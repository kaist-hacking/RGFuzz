use crate::{instantiate::CompiledModule, AsContext, Module};
#[allow(unused_imports)]
use anyhow::bail;
use anyhow::Result;
#[cfg(feature = "profiling")]
use fxprof_processed_profile::{
    debugid::DebugId, CategoryHandle, CpuDelta, Frame, FrameFlags, FrameInfo, LibraryInfo, Profile,
    ReferenceTimestamp, Symbol, SymbolTable, Timestamp,
};
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};
use wasmtime_environ::demangle_function_name;
use wasmtime_runtime::Backtrace;

// TODO: collect more data
// - Provide additional hooks for recording host-guest transitions, to be
//   invoked from a Store::call_hook
// - On non-Windows, measure thread-local CPU usage between events with
//   rustix::time::clock_gettime(ClockId::ThreadCPUTime)
// - Report which wasm module, and maybe instance, each frame came from

/// Collects basic profiling data for a single WebAssembly guest.
///
/// This profiler can't provide measurements that are as accurate or detailed
/// as a platform-specific profiler, such as `perf` on Linux. On the other
/// hand, this profiler works on every platform that Wasmtime supports. Also,
/// as an embedder you can use this profiler selectively on individual guest
/// instances rather than profiling the entire process.
///
/// To use this, you'll need to arrange to call [`GuestProfiler::sample`] at
/// regular intervals while the guest is on the stack. The most straightforward
/// way to do that is to call it from a callback registered with
/// [`Store::epoch_deadline_callback()`](crate::Store::epoch_deadline_callback).
///
/// # Accuracy
///
/// The data collection granularity is limited by the mechanism you use to
/// interrupt guest execution and collect a profiling sample.
///
/// If you use epoch interruption, then samples will only be collected at
/// function entry points and loop headers. This introduces some bias to the
/// results. In addition, samples will only be taken at times when WebAssembly
/// functions are running, not during host-calls.
///
/// It is technically possible to use fuel interruption instead. That
/// introduces worse bias since samples occur after a certain number of
/// WebAssembly instructions, which can take different amounts of time.
///
/// You may instead be able to use platform-specific methods, such as
/// `setitimer(ITIMER_VIRTUAL, ...)` on POSIX-compliant systems, to sample on
/// a more accurate interval. The only current requirement is that the guest
/// you wish to profile must be on the same stack where you call `sample`,
/// and executing within the same thread. However, the `GuestProfiler::sample`
/// method is not currently async-signal-safe, so doing this correctly is not
/// easy.
///
/// # Security
///
/// Profiles produced using this profiler do not include any configuration
/// details from the host, such as virtual memory addresses, or from any
/// WebAssembly modules that you haven't specifically allowed. So for
/// example, these profiles should be safe to share with untrusted users
/// who have provided untrusted code that you are running in a multi-tenancy
/// environment.
///
/// However, the profile does include byte offsets into the text section of
/// the compiled module, revealing some information about the size of the code
/// generated for each module. For user-provided modules, the user could get
/// the same information by compiling the module for themself using a similar
/// version of Wasmtime on the same target architecture, but for any module
/// where they don't already have the WebAssembly module binary available this
/// could theoretically lead to an undesirable information disclosure. So you
/// should only include user-provided modules in profiles.
#[cfg(feature = "profiling")]
#[derive(Debug)]
pub struct GuestProfiler {
    profile: Profile,
    modules: Vec<(Range<usize>, fxprof_processed_profile::LibraryHandle)>,
    process: fxprof_processed_profile::ProcessHandle,
    thread: fxprof_processed_profile::ThreadHandle,
    start: Instant,
}

#[cfg(feature = "profiling")]
impl GuestProfiler {
    /// Begin profiling a new guest. When this function is called, the current
    /// wall-clock time is recorded as the start time for the guest.
    ///
    /// The `module_name` parameter is recorded in the profile to help identify
    /// where the profile came from.
    ///
    /// The `interval` parameter should match the rate at which you intend
    /// to call `sample`. However, this is used as a hint and not required to
    /// exactly match the real sample rate.
    ///
    /// Only modules which are present in the `modules` vector will appear in
    /// stack traces in this profile. Any stack frames which were executing
    /// host code or functions from other modules will be omitted. See the
    /// "Security" section of the [`GuestProfiler`] documentation for guidance
    /// on what modules should not be included in this list.
    pub fn new(module_name: &str, interval: Duration, modules: Vec<(String, Module)>) -> Self {
        let zero = ReferenceTimestamp::from_millis_since_unix_epoch(0.0);
        let mut profile = Profile::new(module_name, zero, interval.into());

        let mut modules: Vec<_> = modules
            .into_iter()
            .filter_map(|(name, module)| {
                let compiled = module.compiled_module();
                let text = compiled.text().as_ptr_range();
                let address_range = text.start as usize..text.end as usize;
                module_symbols(name, compiled).map(|lib| (address_range, profile.add_lib(lib)))
            })
            .collect();

        modules.sort_unstable_by_key(|(range, _)| range.start);

        profile.set_reference_timestamp(std::time::SystemTime::now().into());
        let process = profile.add_process(module_name, 0, Timestamp::from_nanos_since_reference(0));
        let thread = profile.add_thread(process, 0, Timestamp::from_nanos_since_reference(0), true);
        let start = Instant::now();
        Self {
            profile,
            modules,
            process,
            thread,
            start,
        }
    }

    /// Add a sample to the profile. This function collects a backtrace from
    /// any stack frames for allowed modules on the current stack. It should
    /// typically be called from a callback registered using
    /// [`Store::epoch_deadline_callback()`](crate::Store::epoch_deadline_callback).
    pub fn sample(&mut self, store: impl AsContext) {
        let now = Timestamp::from_nanos_since_reference(
            self.start.elapsed().as_nanos().try_into().unwrap(),
        );

        let backtrace = Backtrace::new(store.as_context().0.vmruntime_limits());
        let frames = backtrace
            .frames()
            // Samply needs to see the oldest frame first, but we list the newest
            // first, so iterate in reverse.
            .rev()
            .filter_map(|frame| {
                // Find the first module whose start address includes this PC.
                let module_idx = self
                    .modules
                    .partition_point(|(range, _)| range.start > frame.pc());
                if let Some((range, lib)) = self.modules.get(module_idx) {
                    if range.contains(&frame.pc()) {
                        return Some(FrameInfo {
                            frame: Frame::RelativeAddressFromReturnAddress(
                                *lib,
                                u32::try_from(frame.pc() - range.start).unwrap(),
                            ),
                            category_pair: CategoryHandle::OTHER.into(),
                            flags: FrameFlags::empty(),
                        });
                    }
                }
                None
            });

        self.profile
            .add_sample(self.thread, now, frames, CpuDelta::ZERO, 1);
    }

    /// When the guest finishes running, call this function to write the
    /// profile to the given `output`. The output is a JSON-formatted object in
    /// the [Firefox "processed profile format"][fmt]. Files in this format may
    /// be visualized at <https://profiler.firefox.com/>.
    ///
    /// [fmt]: https://github.com/firefox-devtools/profiler/blob/main/docs-developer/processed-profile-format.md
    pub fn finish(mut self, output: impl std::io::Write) -> Result<()> {
        let now = Timestamp::from_nanos_since_reference(
            self.start.elapsed().as_nanos().try_into().unwrap(),
        );
        self.profile.set_thread_end_time(self.thread, now);
        self.profile.set_process_end_time(self.process, now);

        serde_json::to_writer(output, &self.profile)?;
        Ok(())
    }
}

#[cfg(feature = "profiling")]
fn module_symbols(name: String, compiled: &CompiledModule) -> Option<LibraryInfo> {
    let symbols = Vec::from_iter(compiled.finished_functions().map(|(defined_idx, _)| {
        let loc = compiled.func_loc(defined_idx);
        let func_idx = compiled.module().func_index(defined_idx);
        let mut name = String::new();
        match compiled.func_name(func_idx) {
            None => name = format!("wasm_function_{}", defined_idx.as_u32()),
            Some(func_name) => demangle_function_name(&mut name, func_name).unwrap(),
        };
        Symbol {
            address: loc.start,
            size: Some(loc.length),
            name,
        }
    }));
    if symbols.is_empty() {
        return None;
    }

    Some(LibraryInfo {
        name,
        debug_name: String::new(),
        path: String::new(),
        debug_path: String::new(),
        debug_id: DebugId::nil(),
        code_id: None,
        arch: None,
        symbol_table: Some(Arc::new(SymbolTable::new(symbols))),
    })
}
