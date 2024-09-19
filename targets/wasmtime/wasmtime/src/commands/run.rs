//! The module that implements the `wasmtime run` command.

#![cfg_attr(
    not(feature = "component-model"),
    allow(irrefutable_let_patterns, unreachable_patterns)
)]

use crate::common::{Profile, RunCommon, RunTarget};

use anyhow::{anyhow, bail, Context as _, Error, Result};
use clap::Parser;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use wasi_common::sync::{ambient_authority, Dir, TcpListener, WasiCtxBuilder};
use wasmtime::{Engine, Func, Module, Store, StoreLimits, Val, ValType};
use wasmtime_wasi::preview2;

#[cfg(feature = "wasi-nn")]
use wasmtime_wasi_nn::WasiNnCtx;

#[cfg(feature = "wasi-threads")]
use wasmtime_wasi_threads::WasiThreadsCtx;

#[cfg(feature = "wasi-http")]
use wasmtime_wasi_http::WasiHttpCtx;

fn parse_env_var(s: &str) -> Result<(String, Option<String>)> {
    let mut parts = s.splitn(2, '=');
    Ok((
        parts.next().unwrap().to_string(),
        parts.next().map(|s| s.to_string()),
    ))
}

fn parse_dirs(s: &str) -> Result<(String, String)> {
    let mut parts = s.split("::");
    let host = parts.next().unwrap();
    let guest = match parts.next() {
        Some(guest) => guest,
        None => host,
    };
    Ok((host.into(), guest.into()))
}

fn parse_preloads(s: &str) -> Result<(String, PathBuf)> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("must contain exactly one equals character ('=')");
    }
    Ok((parts[0].into(), parts[1].into()))
}

/// Runs a WebAssembly module
#[derive(Parser, PartialEq)]
pub struct RunCommand {
    #[command(flatten)]
    #[allow(missing_docs)]
    pub run: RunCommon,

    /// Grant access of a host directory to a guest.
    ///
    /// If specified as just `HOST_DIR` then the same directory name on the
    /// host is made available within the guest. If specified as `HOST::GUEST`
    /// then the `HOST` directory is opened and made available as the name
    /// `GUEST` in the guest.
    #[arg(long = "dir", value_name = "HOST_DIR[::GUEST_DIR]", value_parser = parse_dirs)]
    pub dirs: Vec<(String, String)>,

    /// Pass an environment variable to the program.
    ///
    /// The `--env FOO=BAR` form will set the environment variable named `FOO`
    /// to the value `BAR` for the guest program using WASI. The `--env FOO`
    /// form will set the environment variable named `FOO` to the same value it
    /// has in the calling process for the guest, or in other words it will
    /// cause the environment variable `FOO` to be inherited.
    #[arg(long = "env", number_of_values = 1, value_name = "NAME[=VAL]", value_parser = parse_env_var)]
    pub vars: Vec<(String, Option<String>)>,

    /// The name of the function to run
    #[arg(long, value_name = "FUNCTION")]
    pub invoke: Option<String>,

    /// Load the given WebAssembly module before the main module
    #[arg(
        long = "preload",
        number_of_values = 1,
        value_name = "NAME=MODULE_PATH",
        value_parser = parse_preloads,
    )]
    pub preloads: Vec<(String, PathBuf)>,

    /// The WebAssembly module to run and arguments to pass to it.
    ///
    /// Arguments passed to the wasm module will be configured as WASI CLI
    /// arguments unless the `--invoke` CLI argument is passed in which case
    /// arguments will be interpreted as arguments to the function specified.
    #[arg(value_name = "WASM", trailing_var_arg = true, required = true)]
    pub module_and_args: Vec<OsString>,
}

enum CliLinker {
    Core(wasmtime::Linker<Host>),
    #[cfg(feature = "component-model")]
    Component(wasmtime::component::Linker<Host>),
}

impl RunCommand {
    /// Executes the command.
    pub fn execute(mut self) -> Result<()> {
        self.run.common.init_logging()?;

        let mut config = self.run.common.config(None)?;

        if self.run.common.wasm.timeout.is_some() {
            config.epoch_interruption(true);
        }
        match self.run.profile {
            Some(Profile::Native(s)) => {
                config.profiler(s);
            }
            Some(Profile::Guest { .. }) => {
                // Further configured down below as well.
                config.epoch_interruption(true);
            }
            None => {}
        }

        let engine = Engine::new(&config)?;

        // Read the wasm module binary either as `*.wat` or a raw binary.
        let main = self
            .run
            .load_module(&engine, self.module_and_args[0].as_ref())?;

        // Validate coredump-on-trap argument
        if let Some(path) = &self.run.common.debug.coredump {
            if path.contains("%") {
                bail!("the coredump-on-trap path does not support patterns yet.")
            }
        }

        let mut linker = match &main {
            RunTarget::Core(_) => CliLinker::Core(wasmtime::Linker::new(&engine)),
            #[cfg(feature = "component-model")]
            RunTarget::Component(_) => {
                CliLinker::Component(wasmtime::component::Linker::new(&engine))
            }
        };
        if let Some(enable) = self.run.common.wasm.unknown_exports_allow {
            match &mut linker {
                CliLinker::Core(l) => {
                    l.allow_unknown_exports(enable);
                }
                #[cfg(feature = "component-model")]
                CliLinker::Component(_) => {
                    bail!("--allow-unknown-exports not supported with components");
                }
            }
        }

        let host = Host::default();
        let mut store = Store::new(&engine, host);
        self.populate_with_wasi(&mut linker, &mut store, &main)?;

        store.data_mut().limits = self.run.store_limits();
        store.limiter(|t| &mut t.limits);

        // If fuel has been configured, we want to add the configured
        // fuel amount to this store.
        if let Some(fuel) = self.run.common.wasm.fuel {
            store.set_fuel(fuel)?;
        }

        // Load the preload wasm modules.
        let mut modules = Vec::new();
        if let RunTarget::Core(m) = &main {
            modules.push((String::new(), m.clone()));
        }
        for (name, path) in self.preloads.iter() {
            // Read the wasm module binary either as `*.wat` or a raw binary
            let module = match self.run.load_module(&engine, path)? {
                RunTarget::Core(m) => m,
                #[cfg(feature = "component-model")]
                RunTarget::Component(_) => bail!("components cannot be loaded with `--preload`"),
            };
            modules.push((name.clone(), module.clone()));

            // Add the module's functions to the linker.
            match &mut linker {
                #[cfg(feature = "cranelift")]
                CliLinker::Core(linker) => {
                    linker.module(&mut store, name, &module).context(format!(
                        "failed to process preload `{}` at `{}`",
                        name,
                        path.display()
                    ))?;
                }
                #[cfg(not(feature = "cranelift"))]
                CliLinker::Core(_) => {
                    bail!("support for --preload disabled at compile time");
                }
                #[cfg(feature = "component-model")]
                CliLinker::Component(_) => {
                    bail!("--preload cannot be used with components");
                }
            }
        }

        // Load the main wasm module.
        match self
            .load_main_module(&mut store, &mut linker, &main, modules)
            .with_context(|| {
                format!(
                    "failed to run main module `{}`",
                    self.module_and_args[0].to_string_lossy()
                )
            }) {
            Ok(()) => (),
            Err(e) => {
                // Exit the process if Wasmtime understands the error;
                // otherwise, fall back on Rust's default error printing/return
                // code.
                if store.data().preview1_ctx.is_some() {
                    return Err(wasi_common::maybe_exit_on_error(e));
                } else if store.data().preview2_ctx.is_some() {
                    if let Some(exit) = e
                        .downcast_ref::<preview2::I32Exit>()
                        .map(|c| c.process_exit_code())
                    {
                        std::process::exit(exit);
                    }
                    if e.is::<wasmtime::Trap>() {
                        eprintln!("Error: {e:?}");
                        cfg_if::cfg_if! {
                            if #[cfg(unix)] {
                                std::process::exit(rustix::process::EXIT_SIGNALED_SIGABRT);
                            } else if #[cfg(windows)] {
                                // https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/abort?view=vs-2019
                                std::process::exit(3);
                            }
                        }
                    }
                    return Err(e);
                } else {
                    unreachable!("either preview1_ctx or preview2_ctx present")
                }
            }
        }

        Ok(())
    }

    fn compute_preopen_dirs(&self) -> Result<Vec<(String, Dir)>> {
        let mut preopen_dirs = Vec::new();

        for (host, guest) in self.dirs.iter() {
            preopen_dirs.push((
                guest.clone(),
                Dir::open_ambient_dir(host, ambient_authority())
                    .with_context(|| format!("failed to open directory '{}'", host))?,
            ));
        }

        Ok(preopen_dirs)
    }

    fn compute_preopen_sockets(&self) -> Result<Vec<TcpListener>> {
        let mut listeners = vec![];

        for address in &self.run.common.wasi.tcplisten {
            let stdlistener = std::net::TcpListener::bind(address)
                .with_context(|| format!("failed to bind to address '{}'", address))?;

            let _ = stdlistener.set_nonblocking(true)?;

            listeners.push(TcpListener::from_std(stdlistener))
        }
        Ok(listeners)
    }

    fn compute_argv(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();

        for (i, arg) in self.module_and_args.iter().enumerate() {
            // For argv[0], which is the program name. Only include the base
            // name of the main wasm module, to avoid leaking path information.
            let arg = if i == 0 {
                Path::new(arg).components().next_back().unwrap().as_os_str()
            } else {
                arg.as_ref()
            };
            result.push(
                arg.to_str()
                    .ok_or_else(|| anyhow!("failed to convert {arg:?} to utf-8"))?
                    .to_string(),
            );
        }

        Ok(result)
    }

    fn setup_epoch_handler(
        &self,
        store: &mut Store<Host>,
        modules: Vec<(String, Module)>,
    ) -> Result<Box<dyn FnOnce(&mut Store<Host>)>> {
        if let Some(Profile::Guest { path, interval }) = &self.run.profile {
            #[cfg(feature = "profiling")]
            return Ok(self.setup_guest_profiler(store, modules, path, *interval));
            #[cfg(not(feature = "profiling"))]
            {
                let _ = (modules, path, interval);
                bail!("support for profiling disabled at compile time");
            }
        }

        if let Some(timeout) = self.run.common.wasm.timeout {
            store.set_epoch_deadline(1);
            let engine = store.engine().clone();
            thread::spawn(move || {
                thread::sleep(timeout);
                engine.increment_epoch();
            });
        }

        Ok(Box::new(|_store| {}))
    }

    #[cfg(feature = "profiling")]
    fn setup_guest_profiler(
        &self,
        store: &mut Store<Host>,
        modules: Vec<(String, Module)>,
        path: &str,
        interval: std::time::Duration,
    ) -> Box<dyn FnOnce(&mut Store<Host>)> {
        use wasmtime::{AsContextMut, GuestProfiler, UpdateDeadline};

        let module_name = self.module_and_args[0].to_str().unwrap_or("<main module>");
        store.data_mut().guest_profiler =
            Some(Arc::new(GuestProfiler::new(module_name, interval, modules)));

        fn sample(mut store: impl AsContextMut<Data = Host>) {
            let mut profiler = store
                .as_context_mut()
                .data_mut()
                .guest_profiler
                .take()
                .unwrap();
            Arc::get_mut(&mut profiler)
                .expect("profiling doesn't support threads yet")
                .sample(&store);
            store.as_context_mut().data_mut().guest_profiler = Some(profiler);
        }

        if let Some(timeout) = self.run.common.wasm.timeout {
            let mut timeout = (timeout.as_secs_f64() / interval.as_secs_f64()).ceil() as u64;
            assert!(timeout > 0);
            store.epoch_deadline_callback(move |mut store| {
                sample(&mut store);
                timeout -= 1;
                if timeout == 0 {
                    bail!("timeout exceeded");
                }
                Ok(UpdateDeadline::Continue(1))
            });
        } else {
            store.epoch_deadline_callback(move |mut store| {
                sample(&mut store);
                Ok(UpdateDeadline::Continue(1))
            });
        }

        store.set_epoch_deadline(1);
        let engine = store.engine().clone();
        thread::spawn(move || loop {
            thread::sleep(interval);
            engine.increment_epoch();
        });

        let path = path.to_string();
        return Box::new(move |store| {
            let profiler = Arc::try_unwrap(store.data_mut().guest_profiler.take().unwrap())
                .expect("profiling doesn't support threads yet");
            if let Err(e) = std::fs::File::create(&path)
                .map_err(anyhow::Error::new)
                .and_then(|output| profiler.finish(std::io::BufWriter::new(output)))
            {
                eprintln!("failed writing profile at {path}: {e:#}");
            } else {
                eprintln!();
                eprintln!("Profile written to: {path}");
                eprintln!("View this profile at https://profiler.firefox.com/.");
            }
        });
    }

    fn load_main_module(
        &self,
        store: &mut Store<Host>,
        linker: &mut CliLinker,
        module: &RunTarget,
        modules: Vec<(String, Module)>,
    ) -> Result<()> {
        // The main module might be allowed to have unknown imports, which
        // should be defined as traps:
        if self.run.common.wasm.unknown_imports_trap == Some(true) {
            #[cfg(feature = "cranelift")]
            match linker {
                CliLinker::Core(linker) => {
                    linker.define_unknown_imports_as_traps(module.unwrap_core())?;
                }
                _ => bail!("cannot use `--trap-unknown-imports` with components"),
            }
            #[cfg(not(feature = "cranelift"))]
            bail!("support for `unknown-imports-trap` disabled at compile time");
        }

        // ...or as default values.
        if self.run.common.wasm.unknown_imports_default == Some(true) {
            #[cfg(feature = "cranelift")]
            match linker {
                CliLinker::Core(linker) => {
                    linker.define_unknown_imports_as_default_values(module.unwrap_core())?;
                }
                _ => bail!("cannot use `--default-values-unknown-imports` with components"),
            }
            #[cfg(not(feature = "cranelift"))]
            bail!("support for `unknown-imports-trap` disabled at compile time");
        }

        let finish_epoch_handler = self.setup_epoch_handler(store, modules)?;

        let result = match linker {
            CliLinker::Core(linker) => {
                let module = module.unwrap_core();
                let instance = linker.instantiate(&mut *store, &module).context(format!(
                    "failed to instantiate {:?}",
                    self.module_and_args[0]
                ))?;

                // If `_initialize` is present, meaning a reactor, then invoke
                // the function.
                if let Some(func) = instance.get_func(&mut *store, "_initialize") {
                    func.typed::<(), ()>(&store)?.call(&mut *store, ())?;
                }

                // Look for the specific function provided or otherwise look for
                // "" or "_start" exports to run as a "main" function.
                let func = if let Some(name) = &self.invoke {
                    Some(
                        instance
                            .get_func(&mut *store, name)
                            .ok_or_else(|| anyhow!("no func export named `{}` found", name))?,
                    )
                } else {
                    instance
                        .get_func(&mut *store, "")
                        .or_else(|| instance.get_func(&mut *store, "_start"))
                };

                match func {
                    Some(func) => self.invoke_func(store, func),
                    None => Ok(()),
                }
            }
            #[cfg(feature = "component-model")]
            CliLinker::Component(linker) => {
                if self.invoke.is_some() {
                    bail!("using `--invoke` with components is not supported");
                }

                let component = module.unwrap_component();

                let (command, _instance) =
                    preview2::command::sync::Command::instantiate(&mut *store, component, linker)?;
                let result = command
                    .wasi_cli_run()
                    .call_run(&mut *store)
                    .context("failed to invoke `run` function")
                    .map_err(|e| self.handle_core_dump(&mut *store, e));

                // Translate the `Result<(),()>` produced by wasm into a feigned
                // explicit exit here with status 1 if `Err(())` is returned.
                result.and_then(|wasm_result| match wasm_result {
                    Ok(()) => Ok(()),
                    Err(()) => Err(wasmtime_wasi::preview2::I32Exit(1).into()),
                })
            }
        };
        finish_epoch_handler(store);

        result
    }

    fn invoke_func(&self, store: &mut Store<Host>, func: Func) -> Result<()> {
        let ty = func.ty(&store);
        if ty.params().len() > 0 {
            eprintln!(
                "warning: using `--invoke` with a function that takes arguments \
                 is experimental and may break in the future"
            );
        }
        let mut args = self.module_and_args.iter().skip(1);
        let mut values = Vec::new();
        for ty in ty.params() {
            let val = match args.next() {
                Some(s) => s,
                None => {
                    if let Some(name) = &self.invoke {
                        bail!("not enough arguments for `{}`", name)
                    } else {
                        bail!("not enough arguments for command default")
                    }
                }
            };
            let val = val
                .to_str()
                .ok_or_else(|| anyhow!("argument is not valid utf-8: {val:?}"))?;
            values.push(match ty {
                // TODO: integer parsing here should handle hexadecimal notation
                // like `0x0...`, but the Rust standard library currently only
                // parses base-10 representations.
                ValType::I32 => Val::I32(val.parse()?),
                ValType::I64 => Val::I64(val.parse()?),
                ValType::F32 => Val::F32(val.parse::<f32>()?.to_bits()),
                ValType::F64 => Val::F64(val.parse::<f64>()?.to_bits()),
                t => bail!("unsupported argument type {:?}", t),
            });
        }

        // Invoke the function and then afterwards print all the results that came
        // out, if there are any.
        let mut results = vec![Val::null(); ty.results().len()];
        let invoke_res = func
            .call(&mut *store, &values, &mut results)
            .with_context(|| {
                if let Some(name) = &self.invoke {
                    format!("failed to invoke `{}`", name)
                } else {
                    format!("failed to invoke command default")
                }
            });

        if let Err(err) = invoke_res {
            return Err(self.handle_core_dump(&mut *store, err));
        }

        if !results.is_empty() {
            eprintln!(
                "warning: using `--invoke` with a function that returns values \
                 is experimental and may break in the future"
            );
        }

        for result in results {
            match result {
                Val::I32(i) => println!("{}", i),
                Val::I64(i) => println!("{}", i),
                Val::F32(f) => println!("{}", f32::from_bits(f)),
                Val::F64(f) => println!("{}", f64::from_bits(f)),
                Val::ExternRef(_) => println!("<externref>"),
                Val::FuncRef(_) => println!("<funcref>"),
                Val::V128(i) => println!("{}", i.as_u128()),
            }
        }

        Ok(())
    }

    #[cfg(feature = "coredump")]
    fn handle_core_dump(&self, store: &mut Store<Host>, err: Error) -> Error {
        let coredump_path = match &self.run.common.debug.coredump {
            Some(path) => path,
            None => return err,
        };
        if !err.is::<wasmtime::Trap>() {
            return err;
        }
        let source_name = self.module_and_args[0]
            .to_str()
            .unwrap_or_else(|| "unknown");

        if let Err(coredump_err) = write_core_dump(store, &err, &source_name, coredump_path) {
            eprintln!("warning: coredump failed to generate: {}", coredump_err);
            err
        } else {
            err.context(format!("core dumped at {}", coredump_path))
        }
    }

    #[cfg(not(feature = "coredump"))]
    fn handle_core_dump(&self, _store: &mut Store<Host>, err: Error) -> Error {
        err
    }

    /// Populates the given `Linker` with WASI APIs.
    fn populate_with_wasi(
        &self,
        linker: &mut CliLinker,
        store: &mut Store<Host>,
        module: &RunTarget,
    ) -> Result<()> {
        if self.run.common.wasi.common != Some(false) {
            match linker {
                CliLinker::Core(linker) => {
                    match (self.run.common.wasi.preview2, self.run.common.wasi.threads) {
                        // If preview2 is explicitly disabled, or if threads
                        // are enabled, then use the historical preview1
                        // implementation.
                        (Some(false), _) | (None, Some(true)) => {
                            wasi_common::sync::add_to_linker(linker, |host| {
                                host.preview1_ctx.as_mut().unwrap()
                            })?;
                            self.set_preview1_ctx(store)?;
                        }
                        // If preview2 was explicitly requested, always use it.
                        // Otherwise use it so long as threads are disabled.
                        //
                        // Note that for now `preview0` is currently
                        // default-enabled but this may turn into
                        // default-disabled in the future.
                        (Some(true), _) | (None, Some(false) | None) => {
                            if self.run.common.wasi.preview0 != Some(false) {
                                preview2::preview0::add_to_linker_sync(linker)?;
                            }
                            preview2::preview1::add_to_linker_sync(linker)?;
                            self.set_preview2_ctx(store)?;
                        }
                    }
                }
                #[cfg(feature = "component-model")]
                CliLinker::Component(linker) => {
                    preview2::command::sync::add_to_linker(linker)?;
                    self.set_preview2_ctx(store)?;
                }
            }
        }

        if self.run.common.wasi.nn == Some(true) {
            #[cfg(not(feature = "wasi-nn"))]
            {
                bail!("Cannot enable wasi-nn when the binary is not compiled with this feature.");
            }
            #[cfg(feature = "wasi-nn")]
            {
                match linker {
                    CliLinker::Core(linker) => {
                        wasmtime_wasi_nn::witx::add_to_linker(linker, |host| {
                            // This WASI proposal is currently not protected against
                            // concurrent access--i.e., when wasi-threads is actively
                            // spawning new threads, we cannot (yet) safely allow access and
                            // fail if more than one thread has `Arc`-references to the
                            // context. Once this proposal is updated (as wasi-common has
                            // been) to allow concurrent access, this `Arc::get_mut`
                            // limitation can be removed.
                            Arc::get_mut(host.wasi_nn.as_mut().unwrap())
                                .expect("wasi-nn is not implemented with multi-threading support")
                        })?;
                    }
                    #[cfg(feature = "component-model")]
                    CliLinker::Component(linker) => {
                        wasmtime_wasi_nn::wit::ML::add_to_linker(linker, |host| {
                            Arc::get_mut(host.wasi_nn.as_mut().unwrap())
                                .expect("wasi-nn is not implemented with multi-threading support")
                        })?;
                    }
                }
                let graphs = self
                    .run
                    .common
                    .wasi
                    .nn_graph
                    .iter()
                    .map(|g| (g.format.clone(), g.dir.clone()))
                    .collect::<Vec<_>>();
                let (backends, registry) = wasmtime_wasi_nn::preload(&graphs)?;
                store.data_mut().wasi_nn = Some(Arc::new(WasiNnCtx::new(backends, registry)));
            }
        }

        if self.run.common.wasi.threads == Some(true) {
            #[cfg(not(feature = "wasi-threads"))]
            {
                // Silence the unused warning for `module` as it is only used in the
                // conditionally-compiled wasi-threads.
                let _ = &module;

                bail!(
                    "Cannot enable wasi-threads when the binary is not compiled with this feature."
                );
            }
            #[cfg(feature = "wasi-threads")]
            {
                let linker = match linker {
                    CliLinker::Core(linker) => linker,
                    _ => bail!("wasi-threads does not support components yet"),
                };
                let module = module.unwrap_core();
                wasmtime_wasi_threads::add_to_linker(linker, store, &module, |host| {
                    host.wasi_threads.as_ref().unwrap()
                })?;
                store.data_mut().wasi_threads = Some(Arc::new(WasiThreadsCtx::new(
                    module.clone(),
                    Arc::new(linker.clone()),
                )?));
            }
        }

        if self.run.common.wasi.http == Some(true) {
            #[cfg(not(all(feature = "wasi-http", feature = "component-model")))]
            {
                bail!("Cannot enable wasi-http when the binary is not compiled with this feature.");
            }
            #[cfg(all(feature = "wasi-http", feature = "component-model"))]
            {
                match linker {
                    CliLinker::Core(_) => {
                        bail!("Cannot enable wasi-http for core wasm modules");
                    }
                    CliLinker::Component(linker) => {
                        wasmtime_wasi_http::proxy::sync::add_only_http_to_linker(linker)?;
                    }
                }

                store.data_mut().wasi_http = Some(Arc::new(WasiHttpCtx {}));
            }
        }

        Ok(())
    }

    fn set_preview1_ctx(&self, store: &mut Store<Host>) -> Result<()> {
        let mut builder = WasiCtxBuilder::new();
        builder.inherit_stdio().args(&self.compute_argv()?)?;

        for (key, value) in self.vars.iter() {
            let value = match value {
                Some(value) => value.clone(),
                None => std::env::var(key)
                    .map_err(|_| anyhow!("environment variable `{key}` not found"))?,
            };
            builder.env(key, &value)?;
        }

        let mut num_fd: usize = 3;

        if self.run.common.wasi.listenfd == Some(true) {
            num_fd = ctx_set_listenfd(num_fd, &mut builder)?;
        }

        for listener in self.compute_preopen_sockets()? {
            builder.preopened_socket(num_fd as _, listener)?;
            num_fd += 1;
        }

        for (name, dir) in self.compute_preopen_dirs()? {
            builder.preopened_dir(dir, name)?;
        }

        store.data_mut().preview1_ctx = Some(builder.build());
        Ok(())
    }

    fn set_preview2_ctx(&self, store: &mut Store<Host>) -> Result<()> {
        let mut builder = preview2::WasiCtxBuilder::new();
        builder.inherit_stdio().args(&self.compute_argv()?);

        for (key, value) in self.vars.iter() {
            let value = match value {
                Some(value) => value.clone(),
                None => std::env::var(key)
                    .map_err(|_| anyhow!("environment variable `{key}` not found"))?,
            };
            builder.env(key, &value);
        }

        if self.run.common.wasi.listenfd == Some(true) {
            bail!("components do not support --listenfd");
        }
        for _ in self.compute_preopen_sockets()? {
            bail!("components do not support --tcplisten");
        }

        for (name, dir) in self.compute_preopen_dirs()? {
            builder.preopened_dir(
                dir,
                preview2::DirPerms::all(),
                preview2::FilePerms::all(),
                name,
            );
        }

        if self.run.common.wasi.inherit_network == Some(true) {
            builder.inherit_network();
        }
        if let Some(enable) = self.run.common.wasi.allow_ip_name_lookup {
            builder.allow_ip_name_lookup(enable);
        }
        if let Some(enable) = self.run.common.wasi.tcp {
            builder.allow_tcp(enable);
        }
        if let Some(enable) = self.run.common.wasi.udp {
            builder.allow_udp(enable);
        }

        let ctx = builder.build();
        store.data_mut().preview2_ctx = Some(Arc::new(Mutex::new(ctx)));
        Ok(())
    }
}

#[derive(Default, Clone)]
struct Host {
    preview1_ctx: Option<wasi_common::WasiCtx>,

    // The Mutex is only needed to satisfy the Sync constraint but we never
    // actually perform any locking on it as we use Mutex::get_mut for every
    // access.
    preview2_ctx: Option<Arc<Mutex<preview2::WasiCtx>>>,

    // Resource table for preview2 if the `preview2_ctx` is in use, otherwise
    // "just" an empty table.
    preview2_table: Arc<Mutex<wasmtime::component::ResourceTable>>,

    // State necessary for the preview1 implementation of WASI backed by the
    // preview2 host implementation. Only used with the `--preview2` flag right
    // now when running core modules.
    preview2_adapter: Arc<preview2::preview1::WasiPreview1Adapter>,

    #[cfg(feature = "wasi-nn")]
    wasi_nn: Option<Arc<WasiNnCtx>>,
    #[cfg(feature = "wasi-threads")]
    wasi_threads: Option<Arc<WasiThreadsCtx<Host>>>,
    #[cfg(feature = "wasi-http")]
    wasi_http: Option<Arc<WasiHttpCtx>>,
    limits: StoreLimits,
    #[cfg(feature = "profiling")]
    guest_profiler: Option<Arc<wasmtime::GuestProfiler>>,
}

impl preview2::WasiView for Host {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        Arc::get_mut(&mut self.preview2_table)
            .expect("preview2 is not compatible with threads")
            .get_mut()
            .unwrap()
    }

    fn ctx(&mut self) -> &mut preview2::WasiCtx {
        let ctx = self.preview2_ctx.as_mut().unwrap();
        Arc::get_mut(ctx)
            .expect("preview2 is not compatible with threads")
            .get_mut()
            .unwrap()
    }
}

impl preview2::preview1::WasiPreview1View for Host {
    fn adapter(&self) -> &preview2::preview1::WasiPreview1Adapter {
        &self.preview2_adapter
    }

    fn adapter_mut(&mut self) -> &mut preview2::preview1::WasiPreview1Adapter {
        Arc::get_mut(&mut self.preview2_adapter).expect("preview2 is not compatible with threads")
    }
}

#[cfg(feature = "wasi-http")]
impl wasmtime_wasi_http::types::WasiHttpView for Host {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        let ctx = self.wasi_http.as_mut().unwrap();
        Arc::get_mut(ctx).expect("preview2 is not compatible with threads")
    }

    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        Arc::get_mut(&mut self.preview2_table)
            .expect("preview2 is not compatible with threads")
            .get_mut()
            .unwrap()
    }
}

#[cfg(not(unix))]
fn ctx_set_listenfd(num_fd: usize, _builder: &mut WasiCtxBuilder) -> Result<usize> {
    Ok(num_fd)
}

#[cfg(unix)]
fn ctx_set_listenfd(mut num_fd: usize, builder: &mut WasiCtxBuilder) -> Result<usize> {
    use listenfd::ListenFd;

    for env in ["LISTEN_FDS", "LISTEN_FDNAMES"] {
        if let Ok(val) = std::env::var(env) {
            builder.env(env, &val)?;
        }
    }

    let mut listenfd = ListenFd::from_env();

    for i in 0..listenfd.len() {
        if let Some(stdlistener) = listenfd.take_tcp_listener(i)? {
            let _ = stdlistener.set_nonblocking(true)?;
            let listener = TcpListener::from_std(stdlistener);
            builder.preopened_socket((3 + i) as _, listener)?;
            num_fd = 3 + i;
        }
    }

    Ok(num_fd)
}

#[cfg(feature = "coredump")]
fn write_core_dump(
    store: &mut Store<Host>,
    err: &anyhow::Error,
    name: &str,
    path: &str,
) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let core_dump = err
        .downcast_ref::<wasmtime::WasmCoreDump>()
        .expect("should have been configured to capture core dumps");

    let core_dump = core_dump.serialize(store, name);

    let mut core_dump_file =
        File::create(path).context(format!("failed to create file at `{}`", path))?;
    core_dump_file
        .write_all(&core_dump)
        .with_context(|| format!("failed to write core dump file at `{}`", path))?;
    Ok(())
}
