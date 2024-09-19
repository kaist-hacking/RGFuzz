#![cfg(not(miri))]

use super::engine;
use anyhow::Result;
use wasmtime::{
    component::{Component, Linker},
    Store,
};

mod ownership;
mod results;

mod no_imports {
    use super::*;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world no-imports {
                export foo: interface {
                    foo: func();
                }

                export bar: func();
            }
        ",
    });

    #[test]
    fn run() -> Result<()> {
        let engine = engine();

        let component = Component::new(
            &engine,
            r#"
                (component
                    (core module $m
                        (func (export ""))
                    )
                    (core instance $i (instantiate $m))

                    (func $f (export "bar") (canon lift (core func $i "")))

                    (instance $i (export "foo" (func $f)))
                    (export "foo" (instance $i))
                )
            "#,
        )?;

        let linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ());
        let (no_imports, _) = NoImports::instantiate(&mut store, &component, &linker)?;
        no_imports.call_bar(&mut store)?;
        no_imports.foo().call_foo(&mut store)?;
        Ok(())
    }
}

mod one_import {
    use super::*;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world one-import {
                import foo: interface {
                    foo: func();
                }

                export bar: func();
            }
        ",
    });

    #[test]
    fn run() -> Result<()> {
        let engine = engine();

        let component = Component::new(
            &engine,
            r#"
                (component
                    (import "foo" (instance $i
                        (export "foo" (func))
                    ))
                    (core module $m
                        (import "" "" (func))
                        (export "" (func 0))
                    )
                    (core func $f (canon lower (func $i "foo")))
                    (core instance $i (instantiate $m
                        (with "" (instance (export "" (func $f))))
                    ))

                    (func $f (export "bar") (canon lift (core func $i "")))
                )
            "#,
        )?;

        #[derive(Default)]
        struct MyImports {
            hit: bool,
        }

        impl foo::Host for MyImports {
            fn foo(&mut self) -> Result<()> {
                self.hit = true;
                Ok(())
            }
        }

        let mut linker = Linker::new(&engine);
        foo::add_to_linker(&mut linker, |f: &mut MyImports| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let (one_import, _) = OneImport::instantiate(&mut store, &component, &linker)?;
        one_import.call_bar(&mut store)?;
        assert!(store.data().hit);
        Ok(())
    }
}

mod resources_at_world_level {
    use super::*;
    use wasmtime::component::Resource;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world resources {
                resource x {
                    constructor();
                }

                export y: func(x: x);
            }
        ",
    });

    #[test]
    fn run() -> Result<()> {
        let engine = engine();

        let component = Component::new(
            &engine,
            r#"
                (component
                    (import "x" (type $x (sub resource)))
                    (import "[constructor]x" (func $ctor (result (own $x))))

                    (core func $dtor (canon resource.drop $x))
                    (core func $ctor (canon lower (func $ctor)))

                    (core module $m
                        (import "" "ctor" (func $ctor (result i32)))
                        (import "" "dtor" (func $dtor (param i32)))

                        (func (export "x") (param i32)
                            (call $dtor (local.get 0))
                            (call $dtor (call $ctor))
                        )
                    )
                    (core instance $i (instantiate $m
                        (with "" (instance
                            (export "ctor" (func $ctor))
                            (export "dtor" (func $dtor))
                        ))
                    ))
                    (func (export "y") (param "x" (own $x))
                        (canon lift (core func $i "x")))
                )
            "#,
        )?;

        #[derive(Default)]
        struct MyImports {
            ctor_hit: bool,
            drops: usize,
        }

        impl HostX for MyImports {
            fn new(&mut self) -> Result<Resource<X>> {
                self.ctor_hit = true;
                Ok(Resource::new_own(80))
            }

            fn drop(&mut self, val: Resource<X>) -> Result<()> {
                match self.drops {
                    0 => assert_eq!(val.rep(), 40),
                    1 => assert_eq!(val.rep(), 80),
                    _ => unreachable!(),
                }
                self.drops += 1;
                Ok(())
            }
        }

        impl ResourcesImports for MyImports {}

        let mut linker = Linker::new(&engine);
        Resources::add_to_linker(&mut linker, |f: &mut MyImports| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let (one_import, _) = Resources::instantiate(&mut store, &component, &linker)?;
        one_import.call_y(&mut store, Resource::new_own(40))?;
        assert!(store.data().ctor_hit);
        assert_eq!(store.data().drops, 2);
        Ok(())
    }
}

mod resources_at_interface_level {
    use super::*;
    use wasmtime::component::Resource;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            interface def {
                resource x {
                    constructor();
                }
            }

            interface user {
                use def.{x};

                y: func(x: x);
            }

            world resources {
                export user;
            }
        ",
    });

    #[test]
    fn run() -> Result<()> {
        let engine = engine();

        let component = Component::new(
            &engine,
            r#"
                (component
                    (import (interface "foo:foo/def") (instance $i
                        (export $x "x" (type (sub resource)))
                        (export "[constructor]x" (func (result (own $x))))
                    ))
                    (alias export $i "x" (type $x))
                    (core func $dtor (canon resource.drop $x))
                    (core func $ctor (canon lower (func $i "[constructor]x")))

                    (core module $m
                        (import "" "ctor" (func $ctor (result i32)))
                        (import "" "dtor" (func $dtor (param i32)))

                        (func (export "x") (param i32)
                            (call $dtor (local.get 0))
                            (call $dtor (call $ctor))
                        )
                    )
                    (core instance $i (instantiate $m
                        (with "" (instance
                            (export "ctor" (func $ctor))
                            (export "dtor" (func $dtor))
                        ))
                    ))
                    (func $y (param "x" (own $x))
                        (canon lift (core func $i "x")))

                    (instance (export (interface "foo:foo/user"))
                        (export "y" (func $y))
                    )
                )
            "#,
        )?;

        #[derive(Default)]
        struct MyImports {
            ctor_hit: bool,
            drops: usize,
        }

        use foo::foo::def::X;

        impl foo::foo::def::HostX for MyImports {
            fn new(&mut self) -> Result<Resource<X>> {
                self.ctor_hit = true;
                Ok(Resource::new_own(80))
            }

            fn drop(&mut self, val: Resource<X>) -> Result<()> {
                match self.drops {
                    0 => assert_eq!(val.rep(), 40),
                    1 => assert_eq!(val.rep(), 80),
                    _ => unreachable!(),
                }
                self.drops += 1;
                Ok(())
            }
        }

        impl foo::foo::def::Host for MyImports {}

        let mut linker = Linker::new(&engine);
        Resources::add_to_linker(&mut linker, |f: &mut MyImports| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let (one_import, _) = Resources::instantiate(&mut store, &component, &linker)?;
        one_import
            .foo_foo_user()
            .call_y(&mut store, Resource::new_own(40))?;
        assert!(store.data().ctor_hit);
        assert_eq!(store.data().drops, 2);
        Ok(())
    }
}

mod async_config {
    use super::*;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world t1 {
                import x: func();
                import y: func();
                export z: func();
            }
        ",
        async: true,
    });

    struct T;

    #[async_trait::async_trait]
    impl T1Imports for T {
        async fn x(&mut self) -> Result<()> {
            Ok(())
        }

        async fn y(&mut self) -> Result<()> {
            Ok(())
        }
    }

    async fn _test_t1(t1: &T1, store: &mut Store<()>) {
        let _ = t1.call_z(&mut *store).await;
    }

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world t2 {
                import x: func();
                import y: func();
                export z: func();
            }
        ",
        async: {
            except_imports: ["x"],
        },
    });

    #[async_trait::async_trait]
    impl T2Imports for T {
        fn x(&mut self) -> Result<()> {
            Ok(())
        }

        async fn y(&mut self) -> Result<()> {
            Ok(())
        }
    }

    async fn _test_t2(t2: &T2, store: &mut Store<()>) {
        let _ = t2.call_z(&mut *store).await;
    }

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world t3 {
                import x: func();
                import y: func();
                export z: func();
            }
        ",
        async: {
            only_imports: ["x"],
        },
    });

    #[async_trait::async_trait]
    impl T3Imports for T {
        async fn x(&mut self) -> Result<()> {
            Ok(())
        }

        fn y(&mut self) -> Result<()> {
            Ok(())
        }
    }

    async fn _test_t3(t3: &T3, store: &mut Store<()>) {
        let _ = t3.call_z(&mut *store).await;
    }
}

mod exported_resources {
    use super::*;
    use std::mem;
    use wasmtime::component::Resource;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            interface a {
                resource x {
                    constructor();
                }
            }

            world resources {
                export b: interface {
                    use a.{x as y};

                    resource x {
                        constructor(y: y);
                        foo: func() -> u32;
                    }
                }

                resource x;

                export f: func(x1: x, x2: x) -> x;
            }
        ",
    });

    #[derive(Default)]
    struct MyImports {
        hostcalls: Vec<Hostcall>,
        next_a_x: u32,
    }

    #[derive(PartialEq, Debug)]
    enum Hostcall {
        DropRootX(u32),
        DropAX(u32),
        NewA,
    }

    use foo::foo::a;

    impl ResourcesImports for MyImports {}

    impl HostX for MyImports {
        fn drop(&mut self, val: Resource<X>) -> Result<()> {
            self.hostcalls.push(Hostcall::DropRootX(val.rep()));
            Ok(())
        }
    }

    impl a::HostX for MyImports {
        fn new(&mut self) -> Result<Resource<a::X>> {
            let rep = self.next_a_x;
            self.next_a_x += 1;
            self.hostcalls.push(Hostcall::NewA);
            Ok(Resource::new_own(rep))
        }

        fn drop(&mut self, val: Resource<a::X>) -> Result<()> {
            self.hostcalls.push(Hostcall::DropAX(val.rep()));
            Ok(())
        }
    }

    impl foo::foo::a::Host for MyImports {}

    #[test]
    fn run() -> Result<()> {
        let engine = engine();

        let component = Component::new(
            &engine,
            r#"
(component
  ;; setup the `foo:foo/a` import
  (import (interface "foo:foo/a") (instance $a
    (export $x "x" (type (sub resource)))
    (export "[constructor]x" (func (result (own $x))))
  ))
  (alias export $a "x" (type $a-x))
  (core func $a-x-drop (canon resource.drop $a-x))
  (core func $a-x-ctor (canon lower (func $a "[constructor]x")))

  ;; setup the root import of the `x` resource
  (import "x" (type $x (sub resource)))
  (core func $root-x-dtor (canon resource.drop $x))

  ;; setup and declare the `x` resource for the `b` export.
  (core module $indirect-dtor
    (func (export "b-x-dtor") (param i32)
      local.get 0
      i32.const 0
      call_indirect (param i32)
    )
    (table (export "$imports") 1 1 funcref)
  )
  (core instance $indirect-dtor (instantiate $indirect-dtor))
  (type $b-x (resource (rep i32) (dtor (func $indirect-dtor "b-x-dtor"))))
  (core func $b-x-drop (canon resource.drop $b-x))
  (core func $b-x-rep (canon resource.rep $b-x))
  (core func $b-x-new (canon resource.new $b-x))

  ;; main module implementation
  (core module $main
    (import "foo:foo/a" "[constructor]x" (func $a-x-ctor (result i32)))
    (import "foo:foo/a" "[resource-drop]x" (func $a-x-dtor (param i32)))
    (import "$root" "[resource-drop]x" (func $x-dtor (param i32)))
    (import "[export]b" "[resource-drop]x" (func $b-x-dtor (param i32)))
    (import "[export]b" "[resource-new]x" (func $b-x-new (param i32) (result i32)))
    (import "[export]b" "[resource-rep]x" (func $b-x-rep (param i32) (result i32)))
    (func (export "b#[constructor]x") (param i32) (result i32)
      (call $a-x-dtor (local.get 0))
      (call $b-x-new (call $a-x-ctor))
    )
    (func (export "b#[method]x.foo") (param i32) (result i32)
      local.get 0)
    (func (export "b#[dtor]x") (param i32)
      (call $a-x-dtor (local.get 0))
    )
    (func (export "f") (param i32 i32) (result i32)
      (call $x-dtor (local.get 0))
      local.get 1
    )
  )
  (core instance $main (instantiate $main
    (with "foo:foo/a" (instance
      (export "[resource-drop]x" (func $a-x-drop))
      (export "[constructor]x" (func $a-x-ctor))
    ))
    (with "$root" (instance
      (export "[resource-drop]x" (func $root-x-dtor))
    ))
    (with "[export]b" (instance
      (export "[resource-drop]x" (func $b-x-drop))
      (export "[resource-rep]x" (func $b-x-rep))
      (export "[resource-new]x" (func $b-x-new))
    ))
  ))

  ;; fill in `$indirect-dtor`'s table with the actual destructor definition
  ;; now that it's available.
  (core module $fixup
    (import "" "b-x-dtor" (func $b-x-dtor (param i32)))
    (import "" "$imports" (table 1 1 funcref))
    (elem (i32.const 0) func $b-x-dtor)
  )
  (core instance (instantiate $fixup
    (with "" (instance
      (export "$imports" (table 0 "$imports"))
      (export "b-x-dtor" (func $main "b#[dtor]x"))
    ))
  ))

  ;; Create the `b` export through a subcomponent instantiation.
  (func $b-x-ctor (param "y" (own $a-x)) (result (own $b-x))
    (canon lift (core func $main "b#[constructor]x")))
  (func $b-x-foo (param "self" (borrow $b-x)) (result u32)
    (canon lift (core func $main "b#[method]x.foo")))
  (component $b
    (import "a-x" (type $y (sub resource)))
    (import "b-x" (type $x' (sub resource)))
    (import "ctor" (func $ctor (param "y" (own $y)) (result (own $x'))))
    (import "foo" (func $foo (param "self" (borrow $x')) (result u32)))
    (export $x "x" (type $x'))
    (export "[constructor]x"
      (func $ctor)
      (func (param "y" (own $y)) (result (own $x))))
    (export "[method]x.foo"
      (func $foo)
      (func (param "self" (borrow $x)) (result u32)))
  )
  (instance (export "b") (instantiate $b
    (with "ctor" (func $b-x-ctor))
    (with "foo" (func $b-x-foo))
    (with "a-x" (type 0 "x"))
    (with "b-x" (type $b-x))
  ))

  ;; Create the `f` export which is a bare function
  (func (export "f") (param "x1" (own $x)) (param "x2" (own $x)) (result (own $x))
    (canon lift (core func $main "f")))
)
            "#,
        )?;

        let mut linker = Linker::new(&engine);
        Resources::add_to_linker(&mut linker, |f: &mut MyImports| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let (i, _) = Resources::instantiate(&mut store, &component, &linker)?;

        // call the root export `f` twice
        let ret = i.call_f(&mut store, Resource::new_own(1), Resource::new_own(2))?;
        assert_eq!(ret.rep(), 2);
        assert_eq!(
            mem::take(&mut store.data_mut().hostcalls),
            [Hostcall::DropRootX(1)]
        );
        let ret = i.call_f(&mut store, Resource::new_own(3), Resource::new_own(4))?;
        assert_eq!(ret.rep(), 4);
        assert_eq!(
            mem::take(&mut store.data_mut().hostcalls),
            [Hostcall::DropRootX(3)]
        );

        // interact with the `b` export
        let b = i.b();
        let b_x = b.x().call_constructor(&mut store, Resource::new_own(5))?;
        assert_eq!(
            mem::take(&mut store.data_mut().hostcalls),
            [Hostcall::DropAX(5), Hostcall::NewA]
        );
        b.x().call_foo(&mut store, b_x.clone())?;
        assert_eq!(mem::take(&mut store.data_mut().hostcalls), []);
        b_x.resource_drop(&mut store)?;
        assert_eq!(
            mem::take(&mut store.data_mut().hostcalls),
            [Hostcall::DropAX(0)],
        );
        Ok(())
    }
}
