use anyhow::Result;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use wasmtime::*;

#[test]
fn link_undefined() -> Result<()> {
    let mut store = Store::<()>::default();
    let linker = Linker::new(store.engine());
    let module = Module::new(store.engine(), r#"(module (import "" "" (func)))"#)?;
    assert!(linker.instantiate(&mut store, &module).is_err());
    let module = Module::new(store.engine(), r#"(module (import "" "" (global i32)))"#)?;
    assert!(linker.instantiate(&mut store, &module).is_err());
    let module = Module::new(store.engine(), r#"(module (import "" "" (memory 1)))"#)?;
    assert!(linker.instantiate(&mut store, &module).is_err());
    let module = Module::new(
        store.engine(),
        r#"(module (import "" "" (table 1 funcref)))"#,
    )?;
    assert!(linker.instantiate(&mut store, &module).is_err());
    Ok(())
}

#[test]
fn test_unknown_import_error() -> Result<()> {
    let mut store = Store::<()>::default();
    let linker = Linker::new(store.engine());
    let module = Module::new(
        store.engine(),
        r#"(module (import "unknown-module" "unknown-name" (func)))"#,
    )?;
    let err = linker
        .instantiate(&mut store, &module)
        .expect_err("should fail");
    let unknown_import: UnknownImportError = err.downcast()?;
    assert_eq!(unknown_import.module(), "unknown-module");
    assert_eq!(unknown_import.name(), "unknown-name");
    unknown_import.ty().unwrap_func();
    Ok(())
}

#[test]
fn link_twice_bad() -> Result<()> {
    let mut store = Store::<()>::default();
    let mut linker = Linker::<()>::new(store.engine());

    // functions
    linker.func_wrap("f", "", || {})?;
    assert!(linker.func_wrap("f", "", || {}).is_err());
    assert!(linker
        .func_wrap("f", "", || -> Result<()> { loop {} })
        .is_err());

    // globals
    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let global = Global::new(&mut store, ty, Val::I32(0))?;
    linker.define(&mut store, "g", "1", global.clone())?;
    assert!(linker.define(&mut store, "g", "1", global.clone()).is_err());

    let ty = GlobalType::new(ValType::I32, Mutability::Var);
    let global = Global::new(&mut store, ty, Val::I32(0))?;
    linker.define(&mut store, "g", "2", global.clone())?;
    assert!(linker.define(&mut store, "g", "2", global.clone()).is_err());

    let ty = GlobalType::new(ValType::I64, Mutability::Const);
    let global = Global::new(&mut store, ty, Val::I64(0))?;
    linker.define(&mut store, "g", "3", global.clone())?;
    assert!(linker.define(&mut store, "g", "3", global.clone()).is_err());

    // memories
    let ty = MemoryType::new(1, None);
    let memory = Memory::new(&mut store, ty)?;
    linker.define(&mut store, "m", "", memory.clone())?;
    assert!(linker.define(&mut store, "m", "", memory.clone()).is_err());
    let ty = MemoryType::new(2, None);
    let memory = Memory::new(&mut store, ty)?;
    assert!(linker.define(&mut store, "m", "", memory.clone()).is_err());

    // tables
    let ty = TableType::new(ValType::FuncRef, 1, None);
    let table = Table::new(&mut store, ty, Val::FuncRef(None))?;
    linker.define(&mut store, "t", "", table.clone())?;
    assert!(linker.define(&mut store, "t", "", table.clone()).is_err());
    let ty = TableType::new(ValType::FuncRef, 2, None);
    let table = Table::new(&mut store, ty, Val::FuncRef(None))?;
    assert!(linker.define(&mut store, "t", "", table.clone()).is_err());
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn function_interposition() -> Result<()> {
    let mut store = Store::<()>::default();
    let mut linker = Linker::new(store.engine());
    linker.allow_shadowing(true);
    let mut module = Module::new(
        store.engine(),
        r#"(module (func (export "green") (result i32) (i32.const 7)))"#,
    )?;
    for _ in 0..4 {
        let instance = linker.instantiate(&mut store, &module)?;
        let green = instance.get_export(&mut store, "green").unwrap().clone();
        linker.define(&mut store, "red", "green", green)?;
        module = Module::new(
            store.engine(),
            r#"(module
                (import "red" "green" (func (result i32)))
                (func (export "green") (result i32) (i32.mul (call 0) (i32.const 2)))
            )"#,
        )?;
    }
    let instance = linker.instantiate(&mut store, &module)?;
    let func = instance
        .get_export(&mut store, "green")
        .unwrap()
        .into_func()
        .unwrap();
    let func = func.typed::<(), i32>(&store)?;
    assert_eq!(func.call(&mut store, ())?, 112);
    Ok(())
}

// Same as `function_interposition`, but the linker's name for the function
// differs from the module's name.
#[test]
#[cfg_attr(miri, ignore)]
fn function_interposition_renamed() -> Result<()> {
    let mut store = Store::<()>::default();
    let mut linker = Linker::new(store.engine());
    linker.allow_shadowing(true);
    let mut module = Module::new(
        store.engine(),
        r#"(module (func (export "export") (result i32) (i32.const 7)))"#,
    )?;
    for _ in 0..4 {
        let instance = linker.instantiate(&mut store, &module)?;
        let export = instance.get_export(&mut store, "export").unwrap().clone();
        linker.define(&mut store, "red", "green", export)?;
        module = Module::new(
            store.engine(),
            r#"(module
                (import "red" "green" (func (result i32)))
                (func (export "export") (result i32) (i32.mul (call 0) (i32.const 2)))
            )"#,
        )?;
    }
    let instance = linker.instantiate(&mut store, &module)?;
    let func = instance.get_func(&mut store, "export").unwrap();
    let func = func.typed::<(), i32>(&store)?;
    assert_eq!(func.call(&mut store, ())?, 112);
    Ok(())
}

// Similar to `function_interposition`, but use `Linker::instance` instead of
// `Linker::define`.
#[test]
#[cfg_attr(miri, ignore)]
fn module_interposition() -> Result<()> {
    let mut store = Store::<()>::default();
    let mut linker = Linker::new(store.engine());
    linker.allow_shadowing(true);
    let mut module = Module::new(
        store.engine(),
        r#"(module (func (export "export") (result i32) (i32.const 7)))"#,
    )?;
    for _ in 0..4 {
        let instance = linker.instantiate(&mut store, &module)?;
        linker.instance(&mut store, "instance", instance)?;
        module = Module::new(
            store.engine(),
            r#"(module
                (import "instance" "export" (func (result i32)))
                (func (export "export") (result i32) (i32.mul (call 0) (i32.const 2)))
            )"#,
        )?;
    }
    let instance = linker.instantiate(&mut store, &module)?;
    let func = instance
        .get_export(&mut store, "export")
        .unwrap()
        .into_func()
        .unwrap();
    let func = func.typed::<(), i32>(&store)?;
    assert_eq!(func.call(&mut store, ())?, 112);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn allow_unknown_exports() -> Result<()> {
    let mut store = Store::<()>::default();
    let mut linker = Linker::new(store.engine());
    let module = Module::new(
        store.engine(),
        r#"(module (func (export "_start")) (global (export "g") i32 (i32.const 0)))"#,
    )?;

    assert!(linker.module(&mut store, "module", &module).is_err());

    let mut linker = Linker::new(store.engine());
    linker.allow_unknown_exports(true);
    linker.module(&mut store, "module", &module)?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn no_leak() -> Result<()> {
    struct DropMe(Rc<Cell<bool>>);

    impl Drop for DropMe {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let flag = Rc::new(Cell::new(false));
    {
        let mut store = Store::new(&Engine::default(), DropMe(flag.clone()));
        let mut linker = Linker::new(store.engine());
        let module = Module::new(
            store.engine(),
            r#"
                (module
                    (func (export "_start"))
                )
            "#,
        )?;
        linker.module(&mut store, "a", &module)?;
    }
    assert!(flag.get(), "store was leaked");
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn no_leak_with_imports() -> Result<()> {
    struct DropMe(Arc<AtomicUsize>);

    impl Drop for DropMe {
        fn drop(&mut self) {
            self.0.fetch_add(1, SeqCst);
        }
    }

    let flag = Arc::new(AtomicUsize::new(0));
    {
        let mut store = Store::new(&Engine::default(), DropMe(flag.clone()));
        let mut linker = Linker::new(store.engine());
        let drop_me = DropMe(flag.clone());
        linker.func_wrap("", "", move || {
            let _ = &drop_me;
        })?;
        let module = Module::new(
            store.engine(),
            r#"
                (module
                    (import "" "" (func))
                    (func (export "_start"))
                )
            "#,
        )?;
        linker.module(&mut store, "a", &module)?;
    }
    assert!(flag.load(SeqCst) == 2, "something was leaked");
    Ok(())
}

#[test]
fn get_host_function() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(&engine, r#"(module (import "mod" "f1" (func)))"#)?;

    let mut linker = Linker::new(&engine);
    linker.func_wrap("mod", "f1", || {})?;
    let mut store = Store::new(&engine, ());
    assert!(linker
        .get_by_import(&mut store, &module.imports().nth(0).unwrap())
        .is_some());

    Ok(())
}

#[test]
fn funcs_live_on_to_fight_another_day() -> Result<()> {
    struct DropMe(Arc<AtomicUsize>);

    impl Drop for DropMe {
        fn drop(&mut self) {
            self.0.fetch_add(1, SeqCst);
        }
    }

    let flag = Arc::new(AtomicUsize::new(0));
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    let drop_me = DropMe(flag.clone());
    linker.func_wrap("", "", move || {
        let _ = &drop_me;
    })?;
    assert_eq!(flag.load(SeqCst), 0);

    let get_and_call = || -> Result<()> {
        assert_eq!(flag.load(SeqCst), 0);
        let mut store = Store::new(&engine, ());
        let func = linker.get(&mut store, "", "").unwrap();
        func.into_func().unwrap().call(&mut store, &[], &mut [])?;
        assert_eq!(flag.load(SeqCst), 0);
        Ok(())
    };

    get_and_call()?;
    get_and_call()?;
    drop(linker);
    assert_eq!(flag.load(SeqCst), 1);
    Ok(())
}

#[test]
fn alias_one() -> Result<()> {
    let mut store = Store::<()>::default();
    let mut linker = Linker::new(store.engine());
    assert!(linker.alias("a", "b", "c", "d").is_err());
    linker.func_wrap("a", "b", || {})?;
    assert!(linker.alias("a", "b", "c", "d").is_ok());
    assert!(linker.get(&mut store, "a", "b").is_some());
    assert!(linker.get(&mut store, "c", "d").is_some());
    Ok(())
}

#[test]
fn instance_pre() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    linker.func_wrap("", "", || {})?;

    let module = Module::new(&engine, r#"(module (import "" "" (func)))"#)?;
    let instance_pre = linker.instantiate_pre(&module)?;
    instance_pre.instantiate(&mut Store::new(&engine, ()))?;
    instance_pre.instantiate(&mut Store::new(&engine, ()))?;

    let mut store = Store::new(&engine, ());
    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::I32, Mutability::Const),
        1.into(),
    )?;
    linker.define(&mut store, "", "g", global)?;

    let module = Module::new(
        &engine,
        r#"(module
            (import "" "" (func))
            (import "" "g" (global i32))
        )"#,
    )?;
    let instance_pre = linker.instantiate_pre(&module)?;
    instance_pre.instantiate(&mut store)?;
    instance_pre.instantiate(&mut store)?;
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_trapping_unknown_import() -> Result<()> {
    const WAT: &str = r#"
    (module
      (type $t0 (func))
      (import "" "imp" (func $.imp (type $t0)))
      (func $run call $.imp)
      (func $other)
      (export "run" (func $run))
      (export "other" (func $other))
    )
    "#;

    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), WAT).expect("failed to create module");
    let mut linker = Linker::new(store.engine());

    linker.define_unknown_imports_as_traps(&module)?;
    let instance = linker.instantiate(&mut store, &module)?;

    // "run" calls an import function which will not be defined, so it should trap
    let run_func = instance
        .get_func(&mut store, "run")
        .expect("expected a run func in the module");

    let err = run_func.call(&mut store, &[], &mut []).unwrap_err();
    assert!(err.is::<UnknownImportError>());

    // "other" does not call the import function, so it should not trap
    let other_func = instance
        .get_func(&mut store, "other")
        .expect("expected an other func in the module");

    other_func.call(&mut store, &[], &mut [])?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_default_value_unknown_import() -> Result<()> {
    const WAT: &str = r#"
      (module
        (import "unknown" "func" (func $unknown_func (result i64 f32 externref)))
        (func (export "run") (result i64 f32 externref)
          call $unknown_func
        )
      )
    "#;

    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), WAT).expect("failed to create module");
    let mut linker = Linker::new(store.engine());

    linker.define_unknown_imports_as_default_values(&module)?;
    let instance = linker.instantiate(&mut store, &module)?;

    // "run" calls an import function which will not be defined, so it should
    // return default values.
    let run_func = instance
        .get_func(&mut store, "run")
        .expect("expected a run func in the module");

    let mut results = vec![Val::I32(1), Val::I32(2), Val::I32(3)];
    run_func.call(&mut store, &[], &mut results)?;

    assert_eq!(results[0].i64(), Some(0));
    assert_eq!(results[1].f32(), Some(0.0));
    assert!(results[2].externref().unwrap().is_none());

    Ok(())
}
