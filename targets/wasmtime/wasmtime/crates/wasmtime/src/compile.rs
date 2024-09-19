//! Wasm compilation orchestration.
//!
//! It works roughly like this:
//!
//! * We walk over the Wasm module/component and make a list of all the things
//!   we need to compile. This is a `CompileInputs`.
//!
//! * The `CompileInputs::compile` method compiles each of these in parallel,
//!   producing a `UnlinkedCompileOutputs`. This is an unlinked set of compiled
//!   functions, bucketed by type of function.
//!
//! * The `UnlinkedCompileOutputs::pre_link` method re-arranges the compiled
//!   functions into a flat list. This is the order we will place them within
//!   the ELF file, so we must also keep track of all the functions' indices
//!   within this list, because we will need them for resolving
//!   relocations. These indices are kept track of in the resulting
//!   `FunctionIndices`.
//!
//! * The `FunctionIndices::link_and_append_code` method appends the functions
//!   to the given ELF file and resolves relocations. It produces an `Artifacts`
//!   which contains the data needed at runtime to find and call Wasm
//!   functions. It is up to the caller to serialize the relevant parts of the
//!   `Artifacts` into the ELF file.

use crate::Engine;
use anyhow::{Context, Result};
use std::{
    any::Any,
    collections::{btree_map, BTreeMap, BTreeSet, HashMap},
    mem,
};
#[cfg(feature = "component-model")]
use wasmtime_environ::component::Translator;
use wasmtime_environ::{
    CompiledFunctionInfo, CompiledModuleInfo, Compiler, DefinedFuncIndex, FinishedObject,
    FuncIndex, FunctionBodyData, ModuleEnvironment, ModuleInternedTypeIndex, ModuleTranslation,
    ModuleType, ModuleTypes, ModuleTypesBuilder, ObjectKind, PrimaryMap, StaticModuleIndex,
    WasmFunctionInfo,
};

/// Converts an input binary-encoded WebAssembly module to compilation
/// artifacts and type information.
///
/// This is where compilation actually happens of WebAssembly modules and
/// translation/parsing/validation of the binary input occurs. The binary
/// artifact represented in the `MmapVec` returned here is an in-memory ELF
/// file in an owned area of virtual linear memory where permissions (such
/// as the executable bit) can be applied.
///
/// Additionally compilation returns an `Option` here which is always
/// `Some`, notably compiled metadata about the module in addition to the
/// type information found within.
pub(crate) fn build_artifacts<T: FinishedObject>(
    engine: &Engine,
    wasm: &[u8],
) -> Result<(T, Option<(CompiledModuleInfo, ModuleTypes)>)> {
    let tunables = engine.tunables();

    // First a `ModuleEnvironment` is created which records type information
    // about the wasm module. This is where the WebAssembly is parsed and
    // validated. Afterwards `types` will have all the type information for
    // this module.
    let mut validator = wasmparser::Validator::new_with_features(engine.config().features.clone());
    let parser = wasmparser::Parser::new(0);
    let mut types = Default::default();
    let mut translation = ModuleEnvironment::new(tunables, &mut validator, &mut types)
        .translate(parser, wasm)
        .context("failed to parse WebAssembly module")?;
    let functions = mem::take(&mut translation.function_body_inputs);

    let compile_inputs = CompileInputs::for_module(&types, &translation, functions);
    let unlinked_compile_outputs = compile_inputs.compile(engine)?;
    let types = types.finish();
    let (compiled_funcs, function_indices) = unlinked_compile_outputs.pre_link();

    // Emplace all compiled functions into the object file with any other
    // sections associated with code as well.
    let mut object = engine.compiler().object(ObjectKind::Module)?;
    // Insert `Engine` and type-level information into the compiled
    // artifact so if this module is deserialized later it contains all
    // information necessary.
    //
    // Note that `append_compiler_info` and `append_types` here in theory
    // can both be skipped if this module will never get serialized.
    // They're only used during deserialization and not during runtime for
    // the module itself. Currently there's no need for that, however, so
    // it's left as an exercise for later.
    engine.append_compiler_info(&mut object);
    engine.append_bti(&mut object);

    let (mut object, compilation_artifacts) = function_indices.link_and_append_code(
        object,
        engine,
        compiled_funcs,
        std::iter::once(translation).collect(),
    )?;

    let info = compilation_artifacts.unwrap_as_module_info();
    object.serialize_info(&(&info, &types));
    let result = T::finish_object(object)?;

    Ok((result, Some((info, types))))
}

/// Performs the compilation phase for a component, translating and
/// validating the provided wasm binary to machine code.
///
/// This method will compile all nested core wasm binaries in addition to
/// any necessary extra functions required for operation with components.
/// The output artifact here is the serialized object file contained within
/// an owned mmap along with metadata about the compilation itself.
#[cfg(feature = "component-model")]
pub(crate) fn build_component_artifacts<T: FinishedObject>(
    engine: &Engine,
    binary: &[u8],
) -> Result<(T, wasmtime_environ::component::ComponentArtifacts)> {
    use wasmtime_environ::component::{CompiledComponentInfo, ComponentArtifacts};
    use wasmtime_environ::ScopeVec;

    let tunables = engine.tunables();
    let compiler = engine.compiler();

    let scope = ScopeVec::new();
    let mut validator = wasmparser::Validator::new_with_features(engine.config().features.clone());
    let mut types = Default::default();
    let (component, mut module_translations) =
        Translator::new(tunables, &mut validator, &mut types, &scope)
            .translate(binary)
            .context("failed to parse WebAssembly module")?;

    let compile_inputs = CompileInputs::for_component(
        &types,
        &component,
        module_translations.iter_mut().map(|(i, translation)| {
            let functions = mem::take(&mut translation.function_body_inputs);
            (i, &*translation, functions)
        }),
    );
    let unlinked_compile_outputs = compile_inputs.compile(&engine)?;

    let (compiled_funcs, function_indices) = unlinked_compile_outputs.pre_link();

    let mut object = compiler.object(ObjectKind::Component)?;
    engine.append_compiler_info(&mut object);
    engine.append_bti(&mut object);

    let (mut object, compilation_artifacts) = function_indices.link_and_append_code(
        object,
        engine,
        compiled_funcs,
        module_translations,
    )?;
    let (types, ty) = types.finish(
        &compilation_artifacts.modules,
        component
            .component
            .import_types
            .iter()
            .map(|(_, (name, ty))| (name.clone(), *ty)),
        component
            .component
            .exports
            .iter()
            .map(|(name, ty)| (name.clone(), ty)),
    );

    let info = CompiledComponentInfo {
        component: component.component,
        trampolines: compilation_artifacts.trampolines,
        resource_drop_wasm_to_native_trampoline: compilation_artifacts
            .resource_drop_wasm_to_native_trampoline,
    };
    let artifacts = ComponentArtifacts {
        info,
        ty,
        types,
        static_modules: compilation_artifacts.modules,
    };
    object.serialize_info(&artifacts);

    let result = T::finish_object(object)?;
    Ok((result, artifacts))
}

type CompileInput<'a> = Box<dyn FnOnce(&dyn Compiler) -> Result<CompileOutput> + Send + 'a>;

/// A sortable, comparable key for a compilation output.
///
/// Two `u32`s to align with `cranelift_codegen::ir::UserExternalName`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CompileKey {
    // The namespace field is bitpacked like:
    //
    //     [ kind:i3 module:i29 ]
    namespace: u32,

    index: u32,
}

impl CompileKey {
    const KIND_BITS: u32 = 3;
    const KIND_OFFSET: u32 = 32 - Self::KIND_BITS;
    const KIND_MASK: u32 = ((1 << Self::KIND_BITS) - 1) << Self::KIND_OFFSET;

    fn kind(&self) -> u32 {
        self.namespace & Self::KIND_MASK
    }

    fn module(&self) -> StaticModuleIndex {
        StaticModuleIndex::from_u32(self.namespace & !Self::KIND_MASK)
    }

    const WASM_FUNCTION_KIND: u32 = Self::new_kind(0);
    const ARRAY_TO_WASM_TRAMPOLINE_KIND: u32 = Self::new_kind(1);
    const NATIVE_TO_WASM_TRAMPOLINE_KIND: u32 = Self::new_kind(2);
    const WASM_TO_NATIVE_TRAMPOLINE_KIND: u32 = Self::new_kind(3);

    const fn new_kind(kind: u32) -> u32 {
        assert!(kind < (1 << Self::KIND_BITS));
        kind << Self::KIND_OFFSET
    }

    // NB: more kinds in the other `impl` block.

    fn wasm_function(module: StaticModuleIndex, index: DefinedFuncIndex) -> Self {
        debug_assert_eq!(module.as_u32() & Self::KIND_MASK, 0);
        Self {
            namespace: Self::WASM_FUNCTION_KIND | module.as_u32(),
            index: index.as_u32(),
        }
    }

    fn array_to_wasm_trampoline(module: StaticModuleIndex, index: DefinedFuncIndex) -> Self {
        debug_assert_eq!(module.as_u32() & Self::KIND_MASK, 0);
        Self {
            namespace: Self::ARRAY_TO_WASM_TRAMPOLINE_KIND | module.as_u32(),
            index: index.as_u32(),
        }
    }

    fn native_to_wasm_trampoline(module: StaticModuleIndex, index: DefinedFuncIndex) -> Self {
        debug_assert_eq!(module.as_u32() & Self::KIND_MASK, 0);
        Self {
            namespace: Self::NATIVE_TO_WASM_TRAMPOLINE_KIND | module.as_u32(),
            index: index.as_u32(),
        }
    }

    fn wasm_to_native_trampoline(index: ModuleInternedTypeIndex) -> Self {
        Self {
            namespace: Self::WASM_TO_NATIVE_TRAMPOLINE_KIND,
            index: index.as_u32(),
        }
    }
}

#[cfg(feature = "component-model")]
impl CompileKey {
    const TRAMPOLINE_KIND: u32 = Self::new_kind(4);
    const RESOURCE_DROP_WASM_TO_NATIVE_KIND: u32 = Self::new_kind(5);

    fn trampoline(index: wasmtime_environ::component::TrampolineIndex) -> Self {
        Self {
            namespace: Self::TRAMPOLINE_KIND,
            index: index.as_u32(),
        }
    }

    fn resource_drop_wasm_to_native_trampoline() -> Self {
        Self {
            namespace: Self::RESOURCE_DROP_WASM_TO_NATIVE_KIND,
            index: 0,
        }
    }
}

#[derive(Clone, Copy)]
enum CompiledFunction<T> {
    Function(T),
    #[cfg(feature = "component-model")]
    AllCallFunc(wasmtime_environ::component::AllCallFunc<T>),
}

impl<T> CompiledFunction<T> {
    fn unwrap_function(self) -> T {
        match self {
            Self::Function(f) => f,
            #[cfg(feature = "component-model")]
            Self::AllCallFunc(_) => panic!("CompiledFunction::unwrap_function"),
        }
    }

    #[cfg(feature = "component-model")]
    fn unwrap_all_call_func(self) -> wasmtime_environ::component::AllCallFunc<T> {
        match self {
            Self::AllCallFunc(f) => f,
            Self::Function(_) => panic!("CompiledFunction::unwrap_all_call_func"),
        }
    }
}

#[cfg(feature = "component-model")]
impl<T> From<wasmtime_environ::component::AllCallFunc<T>> for CompiledFunction<T> {
    fn from(f: wasmtime_environ::component::AllCallFunc<T>) -> Self {
        Self::AllCallFunc(f)
    }
}

struct CompileOutput {
    key: CompileKey,
    symbol: String,
    function: CompiledFunction<Box<dyn Any + Send>>,
    info: Option<WasmFunctionInfo>,
}

/// The collection of things we need to compile for a Wasm module or component.
#[derive(Default)]
struct CompileInputs<'a> {
    inputs: Vec<CompileInput<'a>>,
}

impl<'a> CompileInputs<'a> {
    fn push_input(&mut self, f: impl FnOnce(&dyn Compiler) -> Result<CompileOutput> + Send + 'a) {
        self.inputs.push(Box::new(f));
    }

    /// Create the `CompileInputs` for a core Wasm module.
    fn for_module(
        types: &'a ModuleTypesBuilder,
        translation: &'a ModuleTranslation<'a>,
        functions: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'a>>,
    ) -> Self {
        let mut ret = Self::default();
        let module_index = StaticModuleIndex::from_u32(0);

        ret.collect_inputs_in_translations(types, [(module_index, translation, functions)]);

        ret
    }

    /// Create a `CompileInputs` for a component.
    #[cfg(feature = "component-model")]
    fn for_component(
        types: &'a wasmtime_environ::component::ComponentTypesBuilder,
        component: &'a wasmtime_environ::component::ComponentTranslation,
        module_translations: impl IntoIterator<
            Item = (
                StaticModuleIndex,
                &'a ModuleTranslation<'a>,
                PrimaryMap<DefinedFuncIndex, FunctionBodyData<'a>>,
            ),
        >,
    ) -> Self {
        let mut ret = CompileInputs::default();

        ret.collect_inputs_in_translations(types.module_types_builder(), module_translations);

        for (idx, trampoline) in component.trampolines.iter() {
            ret.push_input(move |compiler| {
                Ok(CompileOutput {
                    key: CompileKey::trampoline(idx),
                    symbol: trampoline.symbol_name(),
                    function: compiler
                        .component_compiler()
                        .compile_trampoline(component, types, idx)?
                        .into(),
                    info: None,
                })
            });
        }

        // If there are any resources defined within this component, the
        // signature for `resource.drop` is mentioned somewhere, and the
        // wasm-to-native trampoline for `resource.drop` hasn't been created yet
        // then insert that here. This is possibly required by destruction of
        // resources from the embedder and otherwise won't be explicitly
        // requested through initializers above or such.
        if component.component.num_resources > 0 {
            if let Some(sig) = types.find_resource_drop_signature() {
                ret.push_input(move |compiler| {
                    let trampoline = compiler.compile_wasm_to_native_trampoline(&types[sig])?;
                    Ok(CompileOutput {
                        key: CompileKey::resource_drop_wasm_to_native_trampoline(),
                        symbol: "resource_drop_trampoline".to_string(),
                        function: CompiledFunction::Function(trampoline),
                        info: None,
                    })
                });
            }
        }

        ret
    }

    fn collect_inputs_in_translations(
        &mut self,
        types: &'a ModuleTypesBuilder,
        translations: impl IntoIterator<
            Item = (
                StaticModuleIndex,
                &'a ModuleTranslation<'a>,
                PrimaryMap<DefinedFuncIndex, FunctionBodyData<'a>>,
            ),
        >,
    ) {
        let mut sigs = BTreeSet::new();

        for (module, translation, functions) in translations {
            for (def_func_index, func_body) in functions {
                self.push_input(move |compiler| {
                    let func_index = translation.module.func_index(def_func_index);
                    let (info, function) =
                        compiler.compile_function(translation, def_func_index, func_body, types)?;
                    Ok(CompileOutput {
                        key: CompileKey::wasm_function(module, def_func_index),
                        symbol: format!(
                            "wasm[{}]::function[{}]",
                            module.as_u32(),
                            func_index.as_u32()
                        ),
                        function: CompiledFunction::Function(function),
                        info: Some(info),
                    })
                });

                let func_index = translation.module.func_index(def_func_index);
                if translation.module.functions[func_index].is_escaping() {
                    self.push_input(move |compiler| {
                        let func_index = translation.module.func_index(def_func_index);
                        let trampoline = compiler.compile_array_to_wasm_trampoline(
                            translation,
                            types,
                            def_func_index,
                        )?;
                        Ok(CompileOutput {
                            key: CompileKey::array_to_wasm_trampoline(module, def_func_index),
                            symbol: format!(
                                "wasm[{}]::array_to_wasm_trampoline[{}]",
                                module.as_u32(),
                                func_index.as_u32()
                            ),
                            function: CompiledFunction::Function(trampoline),
                            info: None,
                        })
                    });

                    self.push_input(move |compiler| {
                        let func_index = translation.module.func_index(def_func_index);
                        let trampoline = compiler.compile_native_to_wasm_trampoline(
                            translation,
                            types,
                            def_func_index,
                        )?;
                        Ok(CompileOutput {
                            key: CompileKey::native_to_wasm_trampoline(module, def_func_index),
                            symbol: format!(
                                "wasm[{}]::native_to_wasm_trampoline[{}]",
                                module.as_u32(),
                                func_index.as_u32()
                            ),
                            function: CompiledFunction::Function(trampoline),
                            info: None,
                        })
                    });
                }
            }

            sigs.extend(translation.module.types.iter().map(|(_, ty)| match ty {
                ModuleType::Function(ty) => *ty,
            }));
        }

        for signature in sigs {
            self.push_input(move |compiler| {
                let wasm_func_ty = &types[signature];
                let trampoline = compiler.compile_wasm_to_native_trampoline(wasm_func_ty)?;
                Ok(CompileOutput {
                    key: CompileKey::wasm_to_native_trampoline(signature),
                    symbol: format!(
                        "signatures[{}]::wasm_to_native_trampoline",
                        signature.as_u32()
                    ),
                    function: CompiledFunction::Function(trampoline),
                    info: None,
                })
            });
        }
    }

    /// Compile these `CompileInput`s (maybe in parallel) and return the
    /// resulting `UnlinkedCompileOutput`s.
    fn compile(self, engine: &Engine) -> Result<UnlinkedCompileOutputs> {
        let compiler = engine.compiler();

        // Compile each individual input in parallel.
        let raw_outputs = engine.run_maybe_parallel(self.inputs, |f| f(compiler))?;

        // Bucket the outputs by kind.
        let mut outputs: BTreeMap<u32, Vec<CompileOutput>> = BTreeMap::new();
        for output in raw_outputs {
            outputs.entry(output.key.kind()).or_default().push(output);
        }

        // Assert that the elements within a bucket are all sorted as we expect
        // them to be.
        fn is_sorted_by_key<T, K>(items: &[T], f: impl Fn(&T) -> K) -> bool
        where
            K: PartialOrd,
        {
            items
                .windows(2)
                .all(|window| f(&window[0]) <= f(&window[1]))
        }
        debug_assert!(outputs
            .values()
            .all(|funcs| is_sorted_by_key(funcs, |x| x.key)));

        Ok(UnlinkedCompileOutputs { outputs })
    }
}

#[derive(Default)]
struct UnlinkedCompileOutputs {
    // A map from kind to `CompileOutput`.
    outputs: BTreeMap<u32, Vec<CompileOutput>>,
}

impl UnlinkedCompileOutputs {
    /// Flatten all our functions into a single list and remember each of their
    /// indices within it.
    fn pre_link(self) -> (Vec<(String, Box<dyn Any + Send>)>, FunctionIndices) {
        // The order the functions end up within `compiled_funcs` is the order
        // that they will be laid out in the ELF file, so try and group hot and
        // cold functions together as best we can. However, because we bucket by
        // kind, we shouldn't have any issues with, e.g., cold trampolines
        // appearing in between hot Wasm functions.
        let mut compiled_funcs = vec![];
        let mut indices = FunctionIndices::default();
        for x in self.outputs.into_iter().flat_map(|(_kind, xs)| xs) {
            let index = match x.function {
                CompiledFunction::Function(f) => {
                    let index = compiled_funcs.len();
                    compiled_funcs.push((x.symbol, f));
                    CompiledFunction::Function(index)
                }
                #[cfg(feature = "component-model")]
                CompiledFunction::AllCallFunc(f) => {
                    let array_call = compiled_funcs.len();
                    compiled_funcs.push((format!("{}_array_call", x.symbol), f.array_call));
                    let native_call = compiled_funcs.len();
                    compiled_funcs.push((format!("{}_native_call", x.symbol), f.native_call));
                    let wasm_call = compiled_funcs.len();
                    compiled_funcs.push((format!("{}_wasm_call", x.symbol), f.wasm_call));
                    CompiledFunction::AllCallFunc(wasmtime_environ::component::AllCallFunc {
                        array_call,
                        native_call,
                        wasm_call,
                    })
                }
            };

            if x.key.kind() == CompileKey::WASM_FUNCTION_KIND
                || x.key.kind() == CompileKey::ARRAY_TO_WASM_TRAMPOLINE_KIND
                || x.key.kind() == CompileKey::NATIVE_TO_WASM_TRAMPOLINE_KIND
            {
                indices
                    .compiled_func_index_to_module
                    .insert(index.unwrap_function(), x.key.module());
                if let Some(info) = x.info {
                    indices.wasm_function_infos.insert(x.key, info);
                }
            }

            indices
                .indices
                .entry(x.key.kind())
                .or_default()
                .insert(x.key, index);
        }
        (compiled_funcs, indices)
    }
}

#[derive(Default)]
struct FunctionIndices {
    // A reverse map from an index in `compiled_funcs` to the
    // `StaticModuleIndex` for that function.
    compiled_func_index_to_module: HashMap<usize, StaticModuleIndex>,

    // A map from Wasm functions' compile keys to their infos.
    wasm_function_infos: HashMap<CompileKey, WasmFunctionInfo>,

    // The index of each compiled function, bucketed by compile key kind.
    indices: BTreeMap<u32, BTreeMap<CompileKey, CompiledFunction<usize>>>,
}

impl FunctionIndices {
    /// Link the compiled functions together, resolving relocations, and append
    /// them to the given ELF file.
    fn link_and_append_code<'a>(
        mut self,
        mut obj: object::write::Object<'static>,
        engine: &'a Engine,
        compiled_funcs: Vec<(String, Box<dyn Any + Send>)>,
        translations: PrimaryMap<StaticModuleIndex, ModuleTranslation<'_>>,
    ) -> Result<(wasmtime_environ::ObjectBuilder<'a>, Artifacts)> {
        // Append all the functions to the ELF file.
        //
        // The result is a vector parallel to `compiled_funcs` where
        // `symbol_ids_and_locs[i]` is the symbol ID and function location of
        // `compiled_funcs[i]`.
        let compiler = engine.compiler();
        let tunables = engine.tunables();
        let symbol_ids_and_locs = compiler.append_code(
            &mut obj,
            &compiled_funcs,
            &|caller_index: usize, callee_index: FuncIndex| {
                let module = self
                    .compiled_func_index_to_module
                    .get(&caller_index)
                    .copied()
                    .expect("should only reloc inside wasm function callers");
                let def_func_index = translations[module]
                    .module
                    .defined_func_index(callee_index)
                    .unwrap();
                self.indices[&CompileKey::WASM_FUNCTION_KIND]
                    [&CompileKey::wasm_function(module, def_func_index)]
                    .unwrap_function()
            },
        )?;

        // If requested, generate and add DWARF information.
        if tunables.generate_native_debuginfo &&
            // We can only add DWARF once. Supporting DWARF for components of
            // multiple Wasm modules will require merging the DWARF sections
            // together.
            translations.len() == 1
        {
            for (module, translation) in &translations {
                let funcs: PrimaryMap<_, _> = self
                    .indices
                    .get(&CompileKey::WASM_FUNCTION_KIND)
                    .map(|xs| {
                        xs.range(
                            CompileKey::wasm_function(module, DefinedFuncIndex::from_u32(0))
                                ..=CompileKey::wasm_function(
                                    module,
                                    DefinedFuncIndex::from_u32(u32::MAX - 1),
                                ),
                        )
                    })
                    .into_iter()
                    .flat_map(|x| x)
                    .map(|(_, x)| {
                        let i = x.unwrap_function();
                        (symbol_ids_and_locs[i].0, &*compiled_funcs[i].1)
                    })
                    .collect();
                if !funcs.is_empty() {
                    compiler.append_dwarf(&mut obj, translation, &funcs)?;
                }
            }
        }

        let mut obj = wasmtime_environ::ObjectBuilder::new(obj, tunables);
        let mut artifacts = Artifacts::default();

        // Finally, build our binary artifacts that map things like `FuncIndex`
        // to a function location and all of that using the indices we saved
        // earlier and the function locations we just received after appending
        // the code.

        let mut wasm_functions = self
            .indices
            .remove(&CompileKey::WASM_FUNCTION_KIND)
            .unwrap_or_default()
            .into_iter()
            .peekable();

        fn wasm_functions_for_module(
            wasm_functions: &mut std::iter::Peekable<
                btree_map::IntoIter<CompileKey, CompiledFunction<usize>>,
            >,
            module: StaticModuleIndex,
        ) -> impl Iterator<Item = (CompileKey, CompiledFunction<usize>)> + '_ {
            std::iter::from_fn(move || {
                let (key, _) = wasm_functions.peek()?;
                if key.module() == module {
                    wasm_functions.next()
                } else {
                    None
                }
            })
        }

        let mut array_to_wasm_trampolines = self
            .indices
            .remove(&CompileKey::ARRAY_TO_WASM_TRAMPOLINE_KIND)
            .unwrap_or_default();

        let mut native_to_wasm_trampolines = self
            .indices
            .remove(&CompileKey::NATIVE_TO_WASM_TRAMPOLINE_KIND)
            .unwrap_or_default();

        // NB: unlike the above maps this is not emptied out during iteration
        // since each module may reach into different portions of this map.
        let wasm_to_native_trampolines = self
            .indices
            .remove(&CompileKey::WASM_TO_NATIVE_TRAMPOLINE_KIND)
            .unwrap_or_default();

        artifacts.modules = translations
            .into_iter()
            .map(|(module, mut translation)| {
                // If configured attempt to use static memory initialization which
                // can either at runtime be implemented as a single memcpy to
                // initialize memory or otherwise enabling virtual-memory-tricks
                // such as mmap'ing from a file to get copy-on-write.
                if engine.config().memory_init_cow {
                    let align = compiler.page_size_align();
                    let max_always_allowed = engine.config().memory_guaranteed_dense_image_size;
                    translation.try_static_init(align, max_always_allowed);
                }

                // Attempt to convert table initializer segments to
                // FuncTable representation where possible, to enable
                // table lazy init.
                translation.try_func_table_init();

                let funcs: PrimaryMap<DefinedFuncIndex, CompiledFunctionInfo> =
                    wasm_functions_for_module(&mut wasm_functions, module)
                        .map(|(key, wasm_func_index)| {
                            let wasm_func_index = wasm_func_index.unwrap_function();
                            let wasm_func_loc = symbol_ids_and_locs[wasm_func_index].1;
                            let wasm_func_info = self.wasm_function_infos.remove(&key).unwrap();

                            let array_to_wasm_trampoline = array_to_wasm_trampolines
                                .remove(&CompileKey::array_to_wasm_trampoline(
                                    key.module(),
                                    DefinedFuncIndex::from_u32(key.index),
                                ))
                                .map(|x| symbol_ids_and_locs[x.unwrap_function()].1);

                            let native_to_wasm_trampoline = native_to_wasm_trampolines
                                .remove(&CompileKey::native_to_wasm_trampoline(
                                    key.module(),
                                    DefinedFuncIndex::from_u32(key.index),
                                ))
                                .map(|x| symbol_ids_and_locs[x.unwrap_function()].1);

                            CompiledFunctionInfo {
                                wasm_func_info,
                                wasm_func_loc,
                                array_to_wasm_trampoline,
                                native_to_wasm_trampoline,
                            }
                        })
                        .collect();

                let unique_and_sorted_sigs = translation
                    .module
                    .types
                    .iter()
                    .map(|(_, ty)| match ty {
                        ModuleType::Function(ty) => *ty,
                    })
                    .collect::<BTreeSet<_>>();
                let wasm_to_native_trampolines = unique_and_sorted_sigs
                    .iter()
                    .map(|idx| {
                        let key = CompileKey::wasm_to_native_trampoline(*idx);
                        let compiled = wasm_to_native_trampolines[&key];
                        (*idx, symbol_ids_and_locs[compiled.unwrap_function()].1)
                    })
                    .collect();

                obj.append(translation, funcs, wasm_to_native_trampolines)
            })
            .collect::<Result<PrimaryMap<_, _>>>()?;

        #[cfg(feature = "component-model")]
        {
            artifacts.trampolines = self
                .indices
                .remove(&CompileKey::TRAMPOLINE_KIND)
                .unwrap_or_default()
                .into_iter()
                .map(|(_id, x)| x.unwrap_all_call_func().map(|i| symbol_ids_and_locs[i].1))
                .collect();
            let map = self
                .indices
                .remove(&CompileKey::RESOURCE_DROP_WASM_TO_NATIVE_KIND)
                .unwrap_or_default();
            assert!(map.len() <= 1);
            artifacts.resource_drop_wasm_to_native_trampoline = map
                .into_iter()
                .next()
                .map(|(_id, x)| symbol_ids_and_locs[x.unwrap_function()].1);
        }

        debug_assert!(
            self.indices.is_empty(),
            "Should have processed all compile outputs"
        );

        Ok((obj, artifacts))
    }
}

/// The artifacts necessary for finding and calling Wasm functions at runtime,
/// to be serialized into an ELF file.
#[derive(Default)]
struct Artifacts {
    modules: PrimaryMap<StaticModuleIndex, CompiledModuleInfo>,
    #[cfg(feature = "component-model")]
    trampolines: PrimaryMap<
        wasmtime_environ::component::TrampolineIndex,
        wasmtime_environ::component::AllCallFunc<wasmtime_environ::FunctionLoc>,
    >,
    #[cfg(feature = "component-model")]
    resource_drop_wasm_to_native_trampoline: Option<wasmtime_environ::FunctionLoc>,
}

impl Artifacts {
    /// Assuming this compilation was for a single core Wasm module, get the
    /// resulting `CompiledModuleInfo`.
    fn unwrap_as_module_info(self) -> CompiledModuleInfo {
        assert_eq!(self.modules.len(), 1);
        #[cfg(feature = "component-model")]
        assert!(self.trampolines.is_empty());
        self.modules.into_iter().next().unwrap().1
    }
}
