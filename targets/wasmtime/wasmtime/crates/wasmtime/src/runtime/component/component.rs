use crate::{
    code::CodeObject, code_memory::CodeMemory, instantiate::MmapVecWrapper,
    type_registry::TypeCollection, Engine, Module, ResourcesRequired,
};
use anyhow::{bail, Context, Result};
use std::fs;
use std::mem;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{
    AllCallFunc, CompiledComponentInfo, ComponentArtifacts, ComponentTypes, GlobalInitializer,
    InstantiateModule, StaticModuleIndex, TrampolineIndex, TypeComponentIndex, VMComponentOffsets,
};

use wasmtime_environ::{FunctionLoc, HostPtr, ObjectKind, PrimaryMap};
use wasmtime_runtime::component::ComponentRuntimeInfo;
use wasmtime_runtime::{
    VMArrayCallFunction, VMFuncRef, VMFunctionBody, VMNativeCallFunction, VMWasmCallFunction,
};

/// A compiled WebAssembly Component.
//
// FIXME: need to write more docs here.
#[derive(Clone)]
pub struct Component {
    inner: Arc<ComponentInner>,
}

struct ComponentInner {
    /// Component type index
    ty: TypeComponentIndex,

    /// Core wasm modules that the component defined internally, indexed by the
    /// compile-time-assigned `ModuleUpvarIndex`.
    static_modules: PrimaryMap<StaticModuleIndex, Module>,

    /// Code-related information such as the compiled artifact, type
    /// information, etc.
    ///
    /// Note that the `Arc` here is used to share this allocation with internal
    /// modules.
    code: Arc<CodeObject>,

    /// Metadata produced during compilation.
    info: CompiledComponentInfo,
}

pub(crate) struct AllCallFuncPointers {
    pub wasm_call: NonNull<VMWasmCallFunction>,
    pub array_call: VMArrayCallFunction,
    pub native_call: NonNull<VMNativeCallFunction>,
}

impl Component {
    /// Compiles a new WebAssembly component from the in-memory wasm image
    /// provided.
    //
    // FIXME: need to write more docs here.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(nightlydoc, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn new(engine: &Engine, bytes: impl AsRef<[u8]>) -> Result<Component> {
        let bytes = bytes.as_ref();
        #[cfg(feature = "wat")]
        let bytes = wat::parse_bytes(bytes)?;
        Component::from_binary(engine, &bytes)
    }

    /// Compiles a new WebAssembly component from a wasm file on disk pointed to
    /// by `file`.
    //
    // FIXME: need to write more docs here.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(nightlydoc, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn from_file(engine: &Engine, file: impl AsRef<Path>) -> Result<Component> {
        match Self::new(
            engine,
            &fs::read(&file).with_context(|| "failed to read input file")?,
        ) {
            Ok(m) => Ok(m),
            Err(e) => {
                cfg_if::cfg_if! {
                    if #[cfg(feature = "wat")] {
                        let mut e = e.downcast::<wat::Error>()?;
                        e.set_path(file);
                        bail!(e)
                    } else {
                        Err(e)
                    }
                }
            }
        }
    }

    /// Compiles a new WebAssembly component from the in-memory wasm image
    /// provided.
    //
    // FIXME: need to write more docs here.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(nightlydoc, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn from_binary(engine: &Engine, binary: &[u8]) -> Result<Component> {
        use crate::compile::build_component_artifacts;
        use crate::module::HashedEngineCompileEnv;
        use wasmtime_runtime::MmapVec;

        engine
            .check_compatible_with_native_host()
            .context("compilation settings are not compatible with the native host")?;

        cfg_if::cfg_if! {
            if #[cfg(feature = "cache")] {
                let state = (HashedEngineCompileEnv(engine), binary);
                let (code, artifacts) = wasmtime_cache::ModuleCacheEntry::new(
                    "wasmtime",
                    engine.cache_config(),
                )
                .get_data_raw(
                    &state,

                    // Cache miss, compute the actual artifacts
                    |(engine, wasm)| -> Result<_> {
                        let (mmap, artifacts) = build_component_artifacts::<MmapVecWrapper>(engine.0, wasm)?;
                        let code = publish_mmap(mmap.0)?;
                        Ok((code, Some(artifacts)))
                    },

                    // Implementation of how to serialize artifacts
                    |(_engine, _wasm), (code, _info_and_types)| {
                        Some(code.mmap().to_vec())
                    },

                    // Cache hit, deserialize the provided artifacts
                    |(engine, _wasm), serialized_bytes| {
                        let code = engine.0.load_code_bytes(&serialized_bytes, ObjectKind::Component).ok()?;
                        Some((code, None))
                    },
                )?;
            } else {
                let (mmap, artifacts) = build_component_artifacts::<MmapVecWrapper>(engine, binary)?;
                let artifacts = Some(artifacts);
                let code = publish_mmap(mmap.0)?;
            }
        };

        return Component::from_parts(engine, code, artifacts);

        fn publish_mmap(mmap: MmapVec) -> Result<Arc<CodeMemory>> {
            let mut code = CodeMemory::new(mmap)?;
            code.publish()?;
            Ok(Arc::new(code))
        }
    }

    /// Same as [`Module::deserialize`], but for components.
    ///
    /// Note that the file referenced here must contain contents previously
    /// produced by [`Engine::precompile_component`] or
    /// [`Component::serialize`].
    ///
    /// For more information see the [`Module::deserialize`] method.
    ///
    /// [`Module::deserialize`]: crate::Module::deserialize
    pub unsafe fn deserialize(engine: &Engine, bytes: impl AsRef<[u8]>) -> Result<Component> {
        let code = engine.load_code_bytes(bytes.as_ref(), ObjectKind::Component)?;
        Component::from_parts(engine, code, None)
    }

    /// Same as [`Module::deserialize_file`], but for components.
    ///
    /// For more information see the [`Component::deserialize`] and
    /// [`Module::deserialize_file`] methods.
    ///
    /// [`Module::deserialize_file`]: crate::Module::deserialize_file
    pub unsafe fn deserialize_file(engine: &Engine, path: impl AsRef<Path>) -> Result<Component> {
        let code = engine.load_code_file(path.as_ref(), ObjectKind::Component)?;
        Component::from_parts(engine, code, None)
    }

    /// Final assembly step for a component from its in-memory representation.
    ///
    /// If the `artifacts` are specified as `None` here then they will be
    /// deserialized from `code_memory`.
    fn from_parts(
        engine: &Engine,
        code_memory: Arc<CodeMemory>,
        artifacts: Option<ComponentArtifacts>,
    ) -> Result<Component> {
        let ComponentArtifacts {
            ty,
            info,
            types,
            static_modules,
        } = match artifacts {
            Some(artifacts) => artifacts,
            None => bincode::deserialize(code_memory.wasmtime_info())?,
        };

        // Validate that the component can be used with the current instance
        // allocator.
        engine.allocator().validate_component(
            &info.component,
            &VMComponentOffsets::new(HostPtr, &info.component),
            &|module_index| &static_modules[module_index].module,
        )?;

        // Create a signature registration with the `Engine` for all trampolines
        // and core wasm types found within this component, both for the
        // component and for all included core wasm modules.
        let signatures = TypeCollection::new_for_module(engine.signatures(), types.module_types());

        // Assemble the `CodeObject` artifact which is shared by all core wasm
        // modules as well as the final component.
        let types = Arc::new(types);
        let code = Arc::new(CodeObject::new(code_memory, signatures, types.into()));

        // Convert all information about static core wasm modules into actual
        // `Module` instances by converting each `CompiledModuleInfo`, the
        // `types` type information, and the code memory to a runtime object.
        let static_modules = static_modules
            .into_iter()
            .map(|(_, info)| Module::from_parts_raw(engine, code.clone(), info, false))
            .collect::<Result<_>>()?;

        Ok(Component {
            inner: Arc::new(ComponentInner {
                ty,
                static_modules,
                code,
                info,
            }),
        })
    }

    pub(crate) fn ty(&self) -> TypeComponentIndex {
        self.inner.ty
    }

    pub(crate) fn env_component(&self) -> &wasmtime_environ::component::Component {
        &self.inner.info.component
    }

    pub(crate) fn static_module(&self, idx: StaticModuleIndex) -> &Module {
        &self.inner.static_modules[idx]
    }

    #[inline]
    pub(crate) fn types(&self) -> &Arc<ComponentTypes> {
        self.inner.component_types()
    }

    pub(crate) fn signatures(&self) -> &TypeCollection {
        self.inner.code.signatures()
    }

    pub(crate) fn text(&self) -> &[u8] {
        self.inner.code.code_memory().text()
    }

    pub(crate) fn trampoline_ptrs(&self, index: TrampolineIndex) -> AllCallFuncPointers {
        let AllCallFunc {
            wasm_call,
            array_call,
            native_call,
        } = &self.inner.info.trampolines[index];
        AllCallFuncPointers {
            wasm_call: self.func(wasm_call).cast(),
            array_call: unsafe {
                mem::transmute::<NonNull<VMFunctionBody>, VMArrayCallFunction>(
                    self.func(array_call),
                )
            },
            native_call: self.func(native_call).cast(),
        }
    }

    fn func(&self, loc: &FunctionLoc) -> NonNull<VMFunctionBody> {
        let text = self.text();
        let trampoline = &text[loc.start as usize..][..loc.length as usize];
        NonNull::new(trampoline.as_ptr() as *mut VMFunctionBody).unwrap()
    }

    pub(crate) fn code_object(&self) -> &Arc<CodeObject> {
        &self.inner.code
    }

    /// Same as [`Module::serialize`], except for a component.
    ///
    /// Note that the artifact produced here must be passed to
    /// [`Component::deserialize`] and is not compatible for use with
    /// [`Module`].
    ///
    /// [`Module::serialize`]: crate::Module::serialize
    /// [`Module`]: crate::Module
    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(self.code_object().code_memory().mmap().to_vec())
    }

    pub(crate) fn runtime_info(&self) -> Arc<dyn ComponentRuntimeInfo> {
        self.inner.clone()
    }

    /// Creates a new `VMFuncRef` with all fields filled out for the destructor
    /// specified.
    ///
    /// The `dtor`'s own `VMFuncRef` won't have `wasm_call` filled out but this
    /// component may have `resource_drop_wasm_to_native_trampoline` filled out
    /// if necessary in which case it's filled in here.
    pub(crate) fn resource_drop_func_ref(&self, dtor: &crate::func::HostFunc) -> VMFuncRef {
        // Host functions never have their `wasm_call` filled in at this time.
        assert!(dtor.func_ref().wasm_call.is_none());

        // Note that if `resource_drop_wasm_to_native_trampoline` is not present
        // then this can't be called by the component, so it's ok to leave it
        // blank.
        let wasm_call = self
            .inner
            .info
            .resource_drop_wasm_to_native_trampoline
            .as_ref()
            .map(|i| self.func(i).cast());
        VMFuncRef {
            wasm_call,
            ..*dtor.func_ref()
        }
    }

    /// Returns a summary of the resources required to instantiate this
    /// [`Component`][crate::component::Component].
    ///
    /// Note that when a component imports and instantiates another component or
    /// core module, we cannot determine ahead of time how many resources
    /// instantiating this component will require, and therefore this method
    /// will return `None` in these scenarios.
    ///
    /// Potential uses of the returned information:
    ///
    /// * Determining whether your pooling allocator configuration supports
    ///   instantiating this component.
    ///
    /// * Deciding how many of which `Component` you want to instantiate within
    ///   a fixed amount of resources, e.g. determining whether to create 5
    ///   instances of component X or 10 instances of component Y.
    ///
    /// # Example
    ///
    /// ```
    /// # fn main() -> wasmtime::Result<()> {
    /// use wasmtime::{Config, Engine, component::Component};
    ///
    /// let mut config = Config::new();
    /// config.wasm_multi_memory(true);
    /// config.wasm_component_model(true);
    /// let engine = Engine::new(&config)?;
    ///
    /// let component = Component::new(&engine, &r#"
    ///     (component
    ///         ;; Define a core module that uses two memories.
    ///         (core module $m
    ///             (memory 1)
    ///             (memory 6)
    ///         )
    ///
    ///         ;; Instantiate that core module three times.
    ///         (core instance $i1 (instantiate (module $m)))
    ///         (core instance $i2 (instantiate (module $m)))
    ///         (core instance $i3 (instantiate (module $m)))
    ///     )
    /// "#)?;
    ///
    /// let resources = component.resources_required()
    ///     .expect("this component does not import any core modules or instances");
    ///
    /// // Instantiating the component will require allocating two memories per
    /// // core instance, and there are three instances, so six total memories.
    /// assert_eq!(resources.num_memories, 6);
    /// assert_eq!(resources.max_initial_memory_size, Some(6));
    ///
    /// // The component doesn't need any tables.
    /// assert_eq!(resources.num_tables, 0);
    /// assert_eq!(resources.max_initial_table_size, None);
    /// # Ok(()) }
    /// ```
    pub fn resources_required(&self) -> Option<ResourcesRequired> {
        let mut resources = ResourcesRequired {
            num_memories: 0,
            max_initial_memory_size: None,
            num_tables: 0,
            max_initial_table_size: None,
        };
        for init in &self.env_component().initializers {
            match init {
                GlobalInitializer::InstantiateModule(inst) => match inst {
                    InstantiateModule::Static(index, _) => {
                        let module = self.static_module(*index);
                        resources.add(&module.resources_required());
                    }
                    InstantiateModule::Import(_, _) => {
                        // We can't statically determine the resources required
                        // to instantiate this component.
                        return None;
                    }
                },
                GlobalInitializer::LowerImport { .. }
                | GlobalInitializer::ExtractMemory(_)
                | GlobalInitializer::ExtractRealloc(_)
                | GlobalInitializer::ExtractPostReturn(_)
                | GlobalInitializer::Resource(_) => {}
            }
        }
        Some(resources)
    }
}

impl ComponentRuntimeInfo for ComponentInner {
    fn component(&self) -> &wasmtime_environ::component::Component {
        &self.info.component
    }

    fn component_types(&self) -> &Arc<ComponentTypes> {
        match self.code.types() {
            crate::code::Types::Component(types) => types,
            // The only creator of a `Component` is itself which uses the other
            // variant, so this shouldn't be possible.
            crate::code::Types::Module(_) => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::component::Component;
    use crate::{Config, Engine};
    use wasmtime_environ::MemoryInitialization;

    #[test]
    fn cow_on_by_default() {
        let mut config = Config::new();
        config.wasm_component_model(true);
        let engine = Engine::new(&config).unwrap();
        let component = Component::new(
            &engine,
            r#"
                (component
                    (core module
                        (memory 1)
                        (data (i32.const 100) "abcd")
                    )
                )
            "#,
        )
        .unwrap();

        for (_, module) in component.inner.static_modules.iter() {
            let init = &module.env_module().memory_initialization;
            assert!(matches!(init, MemoryInitialization::Static { .. }));
        }
    }
}
