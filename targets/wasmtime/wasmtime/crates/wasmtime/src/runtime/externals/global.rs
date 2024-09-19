use crate::store::{StoreData, StoreOpaque, Stored};
use crate::trampoline::generate_global_export;
use crate::{AsContext, AsContextMut, ExternRef, Func, GlobalType, Mutability, Val, ValType};
use anyhow::{bail, Result};
use std::mem;
use std::ptr;

/// A WebAssembly `global` value which can be read and written to.
///
/// A `global` in WebAssembly is sort of like a global variable within an
/// [`Instance`](crate::Instance). The `global.get` and `global.set`
/// instructions will modify and read global values in a wasm module. Globals
/// can either be imported or exported from wasm modules.
///
/// A [`Global`] "belongs" to the store that it was originally created within
/// (either via [`Global::new`] or via instantiating a
/// [`Module`](crate::Module)). Operations on a [`Global`] only work with the
/// store it belongs to, and if another store is passed in by accident then
/// methods will panic.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)] // here for the C API
pub struct Global(pub(super) Stored<wasmtime_runtime::ExportGlobal>);

impl Global {
    /// Creates a new WebAssembly `global` value with the provide type `ty` and
    /// initial value `val`.
    ///
    /// The `store` argument will be the owner of the [`Global`] returned. Using
    /// the returned [`Global`] other items in the store may access this global.
    /// For example this could be provided as an argument to
    /// [`Instance::new`](crate::Instance::new) or
    /// [`Linker::define`](crate::Linker::define).
    ///
    /// # Errors
    ///
    /// Returns an error if the `ty` provided does not match the type of the
    /// value `val`, or if `val` comes from a different store than `store`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let mut store = Store::new(&engine, ());
    ///
    /// let ty = GlobalType::new(ValType::I32, Mutability::Const);
    /// let i32_const = Global::new(&mut store, ty, 1i32.into())?;
    /// let ty = GlobalType::new(ValType::F64, Mutability::Var);
    /// let f64_mut = Global::new(&mut store, ty, 2.0f64.into())?;
    ///
    /// let module = Module::new(
    ///     &engine,
    ///     "(module
    ///         (global (import \"\" \"i32-const\") i32)
    ///         (global (import \"\" \"f64-mut\") (mut f64))
    ///     )"
    /// )?;
    ///
    /// let mut linker = Linker::new(&engine);
    /// linker.define(&store, "", "i32-const", i32_const)?;
    /// linker.define(&store, "", "f64-mut", f64_mut)?;
    ///
    /// let instance = linker.instantiate(&mut store, &module)?;
    /// // ...
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(mut store: impl AsContextMut, ty: GlobalType, val: Val) -> Result<Global> {
        Global::_new(store.as_context_mut().0, ty, val)
    }

    fn _new(store: &mut StoreOpaque, ty: GlobalType, val: Val) -> Result<Global> {
        if !val.comes_from_same_store(store) {
            bail!("cross-`Store` globals are not supported");
        }
        if val.ty() != *ty.content() {
            bail!("value provided does not match the type of this global");
        }
        unsafe {
            let wasmtime_export = generate_global_export(store, ty, val);
            Ok(Global::from_wasmtime_global(wasmtime_export, store))
        }
    }

    /// Returns the underlying type of this `global`.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this global.
    pub fn ty(&self, store: impl AsContext) -> GlobalType {
        let store = store.as_context();
        let ty = &store[self.0].global;
        GlobalType::from_wasmtime_global(&ty)
    }

    /// Returns the current [`Val`] of this global.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this global.
    pub fn get(&self, mut store: impl AsContextMut) -> Val {
        unsafe {
            let store = store.as_context_mut();
            let definition = &*store[self.0].definition;
            match self.ty(&store).content() {
                ValType::I32 => Val::from(*definition.as_i32()),
                ValType::I64 => Val::from(*definition.as_i64()),
                ValType::F32 => Val::F32(*definition.as_u32()),
                ValType::F64 => Val::F64(*definition.as_u64()),
                ValType::ExternRef => Val::ExternRef(
                    definition
                        .as_externref()
                        .clone()
                        .map(|inner| ExternRef { inner }),
                ),
                ValType::FuncRef => {
                    Val::FuncRef(Func::from_raw(store, definition.as_func_ref().cast()))
                }
                ValType::V128 => Val::V128((*definition.as_u128()).into()),
            }
        }
    }

    /// Attempts to set the current value of this global to [`Val`].
    ///
    /// # Errors
    ///
    /// Returns an error if this global has a different type than `Val`, if
    /// it's not a mutable global, or if `val` comes from a different store than
    /// the one provided.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this global.
    pub fn set(&self, mut store: impl AsContextMut, val: Val) -> Result<()> {
        let store = store.as_context_mut().0;
        let ty = self.ty(&store);
        if ty.mutability() != Mutability::Var {
            bail!("immutable global cannot be set");
        }
        let ty = ty.content();
        if val.ty() != *ty {
            bail!("global of type {:?} cannot be set to {:?}", ty, val.ty());
        }
        if !val.comes_from_same_store(store) {
            bail!("cross-`Store` values are not supported");
        }
        unsafe {
            let definition = &mut *store[self.0].definition;
            match val {
                Val::I32(i) => *definition.as_i32_mut() = i,
                Val::I64(i) => *definition.as_i64_mut() = i,
                Val::F32(f) => *definition.as_u32_mut() = f,
                Val::F64(f) => *definition.as_u64_mut() = f,
                Val::FuncRef(f) => {
                    *definition.as_func_ref_mut() =
                        f.map_or(ptr::null_mut(), |f| f.vm_func_ref(store).as_ptr().cast());
                }
                Val::ExternRef(x) => {
                    let old = mem::replace(definition.as_externref_mut(), x.map(|x| x.inner));
                    drop(old);
                }
                Val::V128(i) => *definition.as_u128_mut() = i.into(),
            }
        }
        Ok(())
    }

    pub(crate) unsafe fn from_wasmtime_global(
        wasmtime_export: wasmtime_runtime::ExportGlobal,
        store: &mut StoreOpaque,
    ) -> Global {
        Global(store.store_data_mut().insert(wasmtime_export))
    }

    pub(crate) fn wasmtime_ty<'a>(&self, data: &'a StoreData) -> &'a wasmtime_environ::Global {
        &data[self.0].global
    }

    pub(crate) fn vmimport(&self, store: &StoreOpaque) -> wasmtime_runtime::VMGlobalImport {
        wasmtime_runtime::VMGlobalImport {
            from: store[self.0].definition,
        }
    }

    /// Get a stable hash key for this global.
    ///
    /// Even if the same underlying global definition is added to the
    /// `StoreData` multiple times and becomes multiple `wasmtime::Global`s,
    /// this hash key will be consistent across all of these globals.
    pub(crate) fn hash_key(&self, store: &StoreOpaque) -> impl std::hash::Hash + Eq {
        store[self.0].definition as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Instance, Module, Store};

    #[test]
    fn hash_key_is_stable_across_duplicate_store_data_entries() -> Result<()> {
        let mut store = Store::<()>::default();
        let module = Module::new(
            store.engine(),
            r#"
                (module
                    (global (export "g") (mut i32) (i32.const 0))
                )
            "#,
        )?;
        let instance = Instance::new(&mut store, &module, &[])?;

        // Each time we `get_global`, we call `Global::from_wasmtime` which adds
        // a new entry to `StoreData`, so `g1` and `g2` will have different
        // indices into `StoreData`.
        let g1 = instance.get_global(&mut store, "g").unwrap();
        let g2 = instance.get_global(&mut store, "g").unwrap();

        // That said, they really point to the same global.
        assert_eq!(g1.get(&mut store).unwrap_i32(), 0);
        assert_eq!(g2.get(&mut store).unwrap_i32(), 0);
        g1.set(&mut store, Val::I32(42))?;
        assert_eq!(g1.get(&mut store).unwrap_i32(), 42);
        assert_eq!(g2.get(&mut store).unwrap_i32(), 42);

        // And therefore their hash keys are the same.
        assert!(g1.hash_key(&store.as_context().0) == g2.hash_key(&store.as_context().0));

        // But the hash keys are different from different globals.
        let instance2 = Instance::new(&mut store, &module, &[])?;
        let g3 = instance2.get_global(&mut store, "g").unwrap();
        assert!(g1.hash_key(&store.as_context().0) != g3.hash_key(&store.as_context().0));

        Ok(())
    }
}
