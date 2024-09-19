use anyhow::Result;
use cranelift_codegen::isa::unwind::UnwindInfoKind;
use object::write::{Object, SymbolId};
use std::any::Any;
use std::mem;
use std::sync::Mutex;
use wasmparser::FuncValidatorAllocations;
use wasmtime_cranelift_shared::{CompiledFunction, ModuleTextBuilder};
use wasmtime_environ::{
    CompileError, DefinedFuncIndex, FilePos, FuncIndex, FunctionBodyData, FunctionLoc,
    ModuleTranslation, ModuleTypesBuilder, PrimaryMap, TrapEncodingBuilder, VMOffsets,
    WasmFunctionInfo,
};
use winch_codegen::{BuiltinFunctions, TargetIsa, TrampolineKind};

/// Function compilation context.
/// This struct holds information that can be shared globally across
/// all function compilations.
struct CompilationContext {
    /// Validator allocations.
    allocations: FuncValidatorAllocations,
    /// Builtin functions available to JIT code.
    builtins: BuiltinFunctions,
}

pub(crate) struct Compiler {
    isa: Box<dyn TargetIsa>,
    contexts: Mutex<Vec<CompilationContext>>,
}

/// The compiled function environment.
pub struct CompiledFuncEnv;
impl wasmtime_cranelift_shared::CompiledFuncEnv for CompiledFuncEnv {
    fn resolve_user_external_name_ref(
        &self,
        external: cranelift_codegen::ir::UserExternalNameRef,
    ) -> (u32, u32) {
        (0, external.as_u32())
    }
}

impl Compiler {
    pub fn new(isa: Box<dyn TargetIsa>) -> Self {
        Self {
            isa,
            contexts: Mutex::new(Vec::new()),
        }
    }

    /// Get a compilation context or create a new one if none available.
    fn get_context(&self, translation: &ModuleTranslation) -> CompilationContext {
        self.contexts.lock().unwrap().pop().unwrap_or_else(|| {
            let pointer_size = self.isa.pointer_bytes();
            let vmoffsets = VMOffsets::new(pointer_size, &translation.module);
            CompilationContext {
                allocations: Default::default(),
                builtins: BuiltinFunctions::new(&vmoffsets, self.isa.wasmtime_call_conv()),
            }
        })
    }

    /// Save a compilation context.
    fn save_context(&self, mut context: CompilationContext, allocs: FuncValidatorAllocations) {
        context.allocations = allocs;
        self.contexts.lock().unwrap().push(context);
    }

    /// Emit unwind info into the [`CompiledFunction`].
    fn emit_unwind_info(
        &self,
        compiled_function: &mut CompiledFunction<CompiledFuncEnv>,
    ) -> Result<(), CompileError> {
        let kind = match self.isa.triple().operating_system {
            target_lexicon::OperatingSystem::Windows => UnwindInfoKind::Windows,
            _ => UnwindInfoKind::SystemV,
        };

        if let Some(info) = self
            .isa
            .emit_unwind_info(&compiled_function.buffer, kind)
            .map_err(|e| CompileError::Codegen(format!("{e:?}")))?
        {
            compiled_function.set_unwind_info(info);
        }

        Ok(())
    }
}

impl wasmtime_environ::Compiler for Compiler {
    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        index: DefinedFuncIndex,
        data: FunctionBodyData<'_>,
        types: &ModuleTypesBuilder,
    ) -> Result<(WasmFunctionInfo, Box<dyn Any + Send>), CompileError> {
        let index = translation.module.func_index(index);
        let sig = translation.module.functions[index].signature;
        let ty = &types[sig];
        let FunctionBodyData { body, validator } = data;
        let start_srcloc = FilePos::new(
            body.get_binary_reader()
                .original_position()
                .try_into()
                .unwrap(),
        );
        let mut context = self.get_context(translation);
        let mut validator = validator.into_validator(mem::take(&mut context.allocations));
        let buffer = self
            .isa
            .compile_function(
                ty,
                &body,
                translation,
                types,
                &mut context.builtins,
                &mut validator,
            )
            .map_err(|e| CompileError::Codegen(format!("{e:?}")));
        self.save_context(context, validator.into_allocations());
        let buffer = buffer?;

        let mut compiled_function =
            CompiledFunction::new(buffer, CompiledFuncEnv {}, self.isa.function_alignment());

        if self.isa.flags().unwind_info() {
            self.emit_unwind_info(&mut compiled_function)?;
        }

        Ok((
            WasmFunctionInfo {
                start_srcloc,
                stack_maps: Box::new([]),
            },
            Box::new(compiled_function),
        ))
    }

    fn compile_array_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypesBuilder,
        index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let func_index = translation.module.func_index(index);
        let sig = translation.module.functions[func_index].signature;
        let ty = &types[sig];
        let buffer = self
            .isa
            .compile_trampoline(&ty, TrampolineKind::ArrayToWasm(func_index))
            .map_err(|e| CompileError::Codegen(format!("{:?}", e)))?;

        let mut compiled_function =
            CompiledFunction::new(buffer, CompiledFuncEnv {}, self.isa.function_alignment());

        if self.isa.flags().unwind_info() {
            self.emit_unwind_info(&mut compiled_function)?;
        }

        Ok(Box::new(compiled_function))
    }

    fn compile_native_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypesBuilder,
        index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let func_index = translation.module.func_index(index);
        let sig = translation.module.functions[func_index].signature;
        let ty = &types[sig];

        let buffer = self
            .isa
            .compile_trampoline(ty, TrampolineKind::NativeToWasm(func_index))
            .map_err(|e| CompileError::Codegen(format!("{:?}", e)))?;

        let mut compiled_function =
            CompiledFunction::new(buffer, CompiledFuncEnv {}, self.isa.function_alignment());

        if self.isa.flags().unwind_info() {
            self.emit_unwind_info(&mut compiled_function)?;
        }

        Ok(Box::new(compiled_function))
    }

    fn compile_wasm_to_native_trampoline(
        &self,
        wasm_func_ty: &wasmtime_environ::WasmFuncType,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        let buffer = self
            .isa
            .compile_trampoline(wasm_func_ty, TrampolineKind::WasmToNative)
            .map_err(|e| CompileError::Codegen(format!("{:?}", e)))?;

        let mut compiled_function =
            CompiledFunction::new(buffer, CompiledFuncEnv {}, self.isa.function_alignment());

        if self.isa.flags().unwind_info() {
            self.emit_unwind_info(&mut compiled_function)?;
        }

        Ok(Box::new(compiled_function))
    }

    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send>)],
        resolve_reloc: &dyn Fn(usize, FuncIndex) -> usize,
    ) -> Result<Vec<(SymbolId, FunctionLoc)>> {
        let mut builder =
            ModuleTextBuilder::new(obj, self, self.isa.text_section_builder(funcs.len()));
        let mut traps = TrapEncodingBuilder::default();

        let mut ret = Vec::with_capacity(funcs.len());
        for (i, (sym, func)) in funcs.iter().enumerate() {
            let func = func
                .downcast_ref::<CompiledFunction<CompiledFuncEnv>>()
                .unwrap();

            let (sym, range) = builder.append_func(&sym, func, |idx| resolve_reloc(i, idx));
            traps.push(range.clone(), &func.traps().collect::<Vec<_>>());

            let info = FunctionLoc {
                start: u32::try_from(range.start).unwrap(),
                length: u32::try_from(range.end - range.start).unwrap(),
            };
            ret.push((sym, info));
        }
        builder.finish();
        traps.append_to(obj);
        Ok(ret)
    }

    fn emit_trampolines_for_array_call_host_func(
        &self,
        ty: &wasmtime_environ::WasmFuncType,
        // Actually `host_fn: VMArrayCallFunction` but that type is not
        // available in `wasmtime-environ`.
        host_fn: usize,
        obj: &mut Object<'static>,
    ) -> Result<(FunctionLoc, FunctionLoc)> {
        drop((ty, host_fn, obj));
        todo!()
    }

    fn triple(&self) -> &target_lexicon::Triple {
        self.isa.triple()
    }

    fn flags(&self) -> Vec<(&'static str, wasmtime_environ::FlagValue<'static>)> {
        wasmtime_cranelift_shared::clif_flags_to_wasmtime(self.isa.flags().iter())
    }

    fn isa_flags(&self) -> Vec<(&'static str, wasmtime_environ::FlagValue<'static>)> {
        wasmtime_cranelift_shared::clif_flags_to_wasmtime(self.isa.isa_flags())
    }

    fn is_branch_protection_enabled(&self) -> bool {
        self.isa.is_branch_protection_enabled()
    }

    #[cfg(feature = "component-model")]
    fn component_compiler(&self) -> &dyn wasmtime_environ::component::ComponentCompiler {
        todo!()
    }

    fn append_dwarf(
        &self,
        _obj: &mut Object<'_>,
        _translation: &ModuleTranslation<'_>,
        _funcs: &PrimaryMap<DefinedFuncIndex, (SymbolId, &(dyn Any + Send))>,
    ) -> Result<()> {
        todo!()
    }

    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        self.isa.create_systemv_cie()
    }
}
