use crate::preview2::{
    clocks::{
        host::{monotonic_clock, wall_clock},
        HostMonotonicClock, HostWallClock,
    },
    filesystem::{Dir, OpenMode},
    network::{SocketAddrCheck, SocketAddrUse},
    pipe, random, stdio,
    stdio::{StdinStream, StdoutStream},
    DirPerms, FilePerms,
};
use cap_rand::{Rng, RngCore, SeedableRng};
use std::sync::Arc;
use std::{mem, net::SocketAddr};
use wasmtime::component::ResourceTable;

pub struct WasiCtxBuilder {
    stdin: Box<dyn StdinStream>,
    stdout: Box<dyn StdoutStream>,
    stderr: Box<dyn StdoutStream>,
    env: Vec<(String, String)>,
    args: Vec<String>,
    preopens: Vec<(Dir, String)>,
    socket_addr_check: SocketAddrCheck,
    random: Box<dyn RngCore + Send>,
    insecure_random: Box<dyn RngCore + Send>,
    insecure_random_seed: u128,
    wall_clock: Box<dyn HostWallClock + Send>,
    monotonic_clock: Box<dyn HostMonotonicClock + Send>,
    allowed_network_uses: AllowedNetworkUses,
    built: bool,
}

impl WasiCtxBuilder {
    /// Creates a builder for a new context with default parameters set.
    ///
    /// The current defaults are:
    ///
    /// * stdin is closed
    /// * stdout and stderr eat all input but it doesn't go anywhere
    /// * no env vars
    /// * no arguments
    /// * no preopens
    /// * clocks use the host implementation of wall/monotonic clocks
    /// * RNGs are all initialized with random state and suitable generator
    ///   quality to satisfy the requirements of WASI APIs.
    ///
    /// These defaults can all be updated via the various builder configuration
    /// methods below.
    ///
    /// Note that each builder can only be used once to produce a [`WasiCtx`].
    /// Invoking the [`build`](WasiCtxBuilder::build) method will panic on the
    /// second attempt.
    pub fn new() -> Self {
        // For the insecure random API, use `SmallRng`, which is fast. It's
        // also insecure, but that's the deal here.
        let insecure_random = Box::new(
            cap_rand::rngs::SmallRng::from_rng(cap_rand::thread_rng(cap_rand::ambient_authority()))
                .unwrap(),
        );

        // For the insecure random seed, use a `u128` generated from
        // `thread_rng()`, so that it's not guessable from the insecure_random
        // API.
        let insecure_random_seed =
            cap_rand::thread_rng(cap_rand::ambient_authority()).gen::<u128>();
        Self {
            stdin: Box::new(pipe::ClosedInputStream),
            stdout: Box::new(pipe::SinkOutputStream),
            stderr: Box::new(pipe::SinkOutputStream),
            env: Vec::new(),
            args: Vec::new(),
            preopens: Vec::new(),
            socket_addr_check: SocketAddrCheck::default(),
            random: random::thread_rng(),
            insecure_random,
            insecure_random_seed,
            wall_clock: wall_clock(),
            monotonic_clock: monotonic_clock(),
            allowed_network_uses: AllowedNetworkUses::default(),
            built: false,
        }
    }

    pub fn stdin(&mut self, stdin: impl StdinStream + 'static) -> &mut Self {
        self.stdin = Box::new(stdin);
        self
    }

    pub fn stdout(&mut self, stdout: impl StdoutStream + 'static) -> &mut Self {
        self.stdout = Box::new(stdout);
        self
    }

    pub fn stderr(&mut self, stderr: impl StdoutStream + 'static) -> &mut Self {
        self.stderr = Box::new(stderr);
        self
    }

    pub fn inherit_stdin(&mut self) -> &mut Self {
        self.stdin(stdio::stdin())
    }

    pub fn inherit_stdout(&mut self) -> &mut Self {
        self.stdout(stdio::stdout())
    }

    pub fn inherit_stderr(&mut self) -> &mut Self {
        self.stderr(stdio::stderr())
    }

    pub fn inherit_stdio(&mut self) -> &mut Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }

    pub fn envs(&mut self, env: &[(impl AsRef<str>, impl AsRef<str>)]) -> &mut Self {
        self.env.extend(
            env.iter()
                .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned())),
        );
        self
    }

    pub fn env(&mut self, k: impl AsRef<str>, v: impl AsRef<str>) -> &mut Self {
        self.env
            .push((k.as_ref().to_owned(), v.as_ref().to_owned()));
        self
    }

    pub fn args(&mut self, args: &[impl AsRef<str>]) -> &mut Self {
        self.args.extend(args.iter().map(|a| a.as_ref().to_owned()));
        self
    }

    pub fn arg(&mut self, arg: impl AsRef<str>) -> &mut Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    pub fn preopened_dir(
        &mut self,
        dir: cap_std::fs::Dir,
        perms: DirPerms,
        file_perms: FilePerms,
        path: impl AsRef<str>,
    ) -> &mut Self {
        let mut open_mode = OpenMode::empty();
        if perms.contains(DirPerms::READ) {
            open_mode |= OpenMode::READ;
        }
        if perms.contains(DirPerms::MUTATE) {
            open_mode |= OpenMode::WRITE;
        }
        self.preopens.push((
            Dir::new(dir, perms, file_perms, open_mode),
            path.as_ref().to_owned(),
        ));
        self
    }

    /// Set the generator for the secure random number generator to the custom
    /// generator specified.
    ///
    /// Note that contexts have a default RNG configured which is a suitable
    /// generator for WASI and is configured with a random seed per-context.
    ///
    /// Guest code may rely on this random number generator to produce fresh
    /// unpredictable random data in order to maintain its security invariants,
    /// and ideally should use the insecure random API otherwise, so using any
    /// prerecorded or otherwise predictable data may compromise security.
    pub fn secure_random(&mut self, random: impl RngCore + Send + 'static) -> &mut Self {
        self.random = Box::new(random);
        self
    }

    pub fn insecure_random(&mut self, insecure_random: impl RngCore + Send + 'static) -> &mut Self {
        self.insecure_random = Box::new(insecure_random);
        self
    }

    pub fn insecure_random_seed(&mut self, insecure_random_seed: u128) -> &mut Self {
        self.insecure_random_seed = insecure_random_seed;
        self
    }

    pub fn wall_clock(&mut self, clock: impl HostWallClock + 'static) -> &mut Self {
        self.wall_clock = Box::new(clock);
        self
    }

    pub fn monotonic_clock(&mut self, clock: impl HostMonotonicClock + 'static) -> &mut Self {
        self.monotonic_clock = Box::new(clock);
        self
    }

    /// Allow all network addresses accessible to the host
    pub fn inherit_network(&mut self) -> &mut Self {
        self.socket_addr_check(|_, _| true)
    }

    /// A check that will be called for each socket address that is used.
    ///
    /// Returning `true` will permit socket connections to the `SocketAddr`,
    /// while returning `false` will reject the connection.
    pub fn socket_addr_check<F>(&mut self, check: F) -> &mut Self
    where
        F: Fn(&SocketAddr, SocketAddrUse) -> bool + Send + Sync + 'static,
    {
        self.socket_addr_check = SocketAddrCheck(Arc::new(check));
        self
    }

    /// Allow usage of `wasi:sockets/ip-name-lookup`
    pub fn allow_ip_name_lookup(&mut self, enable: bool) -> &mut Self {
        self.allowed_network_uses.ip_name_lookup = enable;
        self
    }

    /// Allow usage of UDP
    pub fn allow_udp(&mut self, enable: bool) -> &mut Self {
        self.allowed_network_uses.udp = enable;
        self
    }

    /// Allow usage of TCP
    pub fn allow_tcp(&mut self, enable: bool) -> &mut Self {
        self.allowed_network_uses.tcp = enable;
        self
    }

    /// Uses the configured context so far to construct the final `WasiCtx`.
    ///
    /// Note that each `WasiCtxBuilder` can only be used to "build" once, and
    /// calling this method twice will panic.
    ///
    /// # Panics
    ///
    /// Panics if this method is called twice.
    pub fn build(&mut self) -> WasiCtx {
        assert!(!self.built);

        let Self {
            stdin,
            stdout,
            stderr,
            env,
            args,
            preopens,
            socket_addr_check,
            random,
            insecure_random,
            insecure_random_seed,
            wall_clock,
            monotonic_clock,
            allowed_network_uses,
            built: _,
        } = mem::replace(self, Self::new());
        self.built = true;

        WasiCtx {
            stdin,
            stdout,
            stderr,
            env,
            args,
            preopens,
            socket_addr_check,
            random,
            insecure_random,
            insecure_random_seed,
            wall_clock,
            monotonic_clock,
            allowed_network_uses,
        }
    }
}

pub trait WasiView: Send {
    fn table(&mut self) -> &mut ResourceTable;
    fn ctx(&mut self) -> &mut WasiCtx;
}

pub struct WasiCtx {
    pub(crate) random: Box<dyn RngCore + Send>,
    pub(crate) insecure_random: Box<dyn RngCore + Send>,
    pub(crate) insecure_random_seed: u128,
    pub(crate) wall_clock: Box<dyn HostWallClock + Send>,
    pub(crate) monotonic_clock: Box<dyn HostMonotonicClock + Send>,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) args: Vec<String>,
    pub(crate) preopens: Vec<(Dir, String)>,
    pub(crate) stdin: Box<dyn StdinStream>,
    pub(crate) stdout: Box<dyn StdoutStream>,
    pub(crate) stderr: Box<dyn StdoutStream>,
    pub(crate) socket_addr_check: SocketAddrCheck,
    pub(crate) allowed_network_uses: AllowedNetworkUses,
}

pub struct AllowedNetworkUses {
    pub ip_name_lookup: bool,
    pub udp: bool,
    pub tcp: bool,
}

impl Default for AllowedNetworkUses {
    fn default() -> Self {
        Self {
            ip_name_lookup: false,
            udp: true,
            tcp: true,
        }
    }
}

impl AllowedNetworkUses {
    pub(crate) fn check_allowed_udp(&self) -> std::io::Result<()> {
        if !self.udp {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "UDP is not allowed",
            ));
        }

        Ok(())
    }

    pub(crate) fn check_allowed_tcp(&self) -> std::io::Result<()> {
        if !self.tcp {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "TCP is not allowed",
            ));
        }

        Ok(())
    }
}
