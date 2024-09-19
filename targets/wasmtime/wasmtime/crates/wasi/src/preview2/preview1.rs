use crate::preview2::bindings::{
    self,
    cli::{
        stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr, terminal_stdin,
        terminal_stdout,
    },
    clocks::{monotonic_clock, wall_clock},
    filesystem::{preopens, types as filesystem},
    io::{poll, streams},
};
use crate::preview2::{FsError, IsATTY, StreamError, StreamResult, WasiView};
use anyhow::{bail, Context};
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashSet};
use std::mem::{self, size_of, size_of_val};
use std::ops::{Deref, DerefMut};
use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use wasmtime::component::Resource;
use wiggle::tracing::instrument;
use wiggle::{GuestError, GuestPtr, GuestStrCow, GuestType};

#[derive(Debug)]
struct File {
    /// The handle to the preview2 descriptor of type [`crate::preview2::filesystem::Descriptor::File`].
    fd: Resource<filesystem::Descriptor>,

    /// The current-position pointer.
    position: Arc<AtomicU64>,

    /// In append mode, all writes append to the file.
    append: bool,

    /// When blocking, read and write calls dispatch to blocking_read and
    /// blocking_check_write on the underlying streams. When false, read and write
    /// dispatch to stream's plain read and check_write.
    blocking_mode: BlockingMode,
}

#[derive(Clone, Copy, Debug)]
enum BlockingMode {
    Blocking,
    NonBlocking,
}
impl BlockingMode {
    fn from_fdflags(flags: &types::Fdflags) -> Self {
        if flags.contains(types::Fdflags::NONBLOCK) {
            BlockingMode::NonBlocking
        } else {
            BlockingMode::Blocking
        }
    }
    async fn read(
        &self,
        host: &mut impl streams::Host,
        input_stream: Resource<streams::InputStream>,
        max_size: usize,
    ) -> Result<Vec<u8>, types::Error> {
        let max_size = max_size.try_into().unwrap_or(u64::MAX);
        match self {
            BlockingMode::Blocking => {
                match streams::HostInputStream::blocking_read(host, input_stream, max_size).await {
                    Ok(r) if r.is_empty() => Err(types::Errno::Intr.into()),
                    Ok(r) => Ok(r),
                    Err(StreamError::Closed) => Ok(Vec::new()),
                    Err(e) => Err(e.into()),
                }
            }

            BlockingMode::NonBlocking => {
                match streams::HostInputStream::read(host, input_stream, max_size).await {
                    Ok(r) => Ok(r),
                    Err(StreamError::Closed) => Ok(Vec::new()),
                    Err(e) => Err(e.into()),
                }
            }
        }
    }
    async fn write(
        &self,
        host: &mut (impl streams::Host + poll::Host),
        output_stream: Resource<streams::OutputStream>,
        bytes: GuestPtr<'_, [u8]>,
    ) -> StreamResult<usize> {
        use streams::HostOutputStream as Streams;

        let bytes = bytes.as_cow().map_err(|e| StreamError::Trap(e.into()))?;
        let mut bytes = &bytes[..];

        match self {
            BlockingMode::Blocking => {
                let total = bytes.len();
                while !bytes.is_empty() {
                    // NOTE: blocking_write_and_flush takes at most one 4k buffer.
                    let len = bytes.len().min(4096);
                    let (chunk, rest) = bytes.split_at(len);
                    bytes = rest;

                    Streams::blocking_write_and_flush(
                        host,
                        output_stream.borrowed(),
                        Vec::from(chunk),
                    )
                    .await?
                }

                Ok(total)
            }
            BlockingMode::NonBlocking => {
                let n = match Streams::check_write(host, output_stream.borrowed()) {
                    Ok(n) => n,
                    Err(StreamError::Closed) => 0,
                    Err(e) => Err(e)?,
                };

                let len = bytes.len().min(n as usize);
                if len == 0 {
                    return Ok(0);
                }

                match Streams::write(host, output_stream.borrowed(), bytes[..len].to_vec()) {
                    Ok(()) => {}
                    Err(StreamError::Closed) => return Ok(0),
                    Err(e) => Err(e)?,
                }

                match Streams::blocking_flush(host, output_stream.borrowed()).await {
                    Ok(()) => {}
                    Err(StreamError::Closed) => return Ok(0),
                    Err(e) => Err(e)?,
                };

                Ok(len)
            }
        }
    }
}

#[derive(Debug)]
enum Descriptor {
    Stdin {
        stream: Resource<streams::InputStream>,
        isatty: IsATTY,
    },
    Stdout {
        stream: Resource<streams::OutputStream>,
        isatty: IsATTY,
    },
    Stderr {
        stream: Resource<streams::OutputStream>,
        isatty: IsATTY,
    },
    /// A fd of type [`crate::preview2::filesystem::Descriptor::Dir`]
    Directory {
        fd: Resource<filesystem::Descriptor>,
        /// The path this directory was preopened as.
        /// `None` means this directory was opened using `open-at`.
        preopen_path: Option<String>,
    },
    /// A fd of type [`crate::preview2::filesystem::Descriptor::File`]
    File(File),
}

#[derive(Debug, Default)]
pub struct WasiPreview1Adapter {
    descriptors: Option<Descriptors>,
}

#[derive(Debug, Default)]
struct Descriptors {
    used: BTreeMap<u32, Descriptor>,
    free: Vec<u32>,
}

impl Deref for Descriptors {
    type Target = BTreeMap<u32, Descriptor>;

    fn deref(&self) -> &Self::Target {
        &self.used
    }
}

impl DerefMut for Descriptors {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.used
    }
}

impl Descriptors {
    /// Initializes [Self] using `preopens`
    fn new(
        host: &mut (impl preopens::Host
                  + stdin::Host
                  + stdout::Host
                  + stderr::Host
                  + terminal_stdin::Host
                  + terminal_stdout::Host
                  + terminal_stderr::Host
                  + terminal_input::Host
                  + terminal_output::Host
                  + ?Sized),
    ) -> Result<Self, types::Error> {
        let mut descriptors = Self::default();
        descriptors.push(Descriptor::Stdin {
            stream: host
                .get_stdin()
                .context("failed to call `get-stdin`")
                .map_err(types::Error::trap)?,
            isatty: if let Some(term_in) = host
                .get_terminal_stdin()
                .context("failed to call `get-terminal-stdin`")
                .map_err(types::Error::trap)?
            {
                terminal_input::HostTerminalInput::drop(host, term_in)
                    .context("failed to call `drop-terminal-input`")
                    .map_err(types::Error::trap)?;
                IsATTY::Yes
            } else {
                IsATTY::No
            },
        })?;
        descriptors.push(Descriptor::Stdout {
            stream: host
                .get_stdout()
                .context("failed to call `get-stdout`")
                .map_err(types::Error::trap)?,
            isatty: if let Some(term_out) = host
                .get_terminal_stdout()
                .context("failed to call `get-terminal-stdout`")
                .map_err(types::Error::trap)?
            {
                terminal_output::HostTerminalOutput::drop(host, term_out)
                    .context("failed to call `drop-terminal-output`")
                    .map_err(types::Error::trap)?;
                IsATTY::Yes
            } else {
                IsATTY::No
            },
        })?;
        descriptors.push(Descriptor::Stderr {
            stream: host
                .get_stderr()
                .context("failed to call `get-stderr`")
                .map_err(types::Error::trap)?,
            isatty: if let Some(term_out) = host
                .get_terminal_stderr()
                .context("failed to call `get-terminal-stderr`")
                .map_err(types::Error::trap)?
            {
                terminal_output::HostTerminalOutput::drop(host, term_out)
                    .context("failed to call `drop-terminal-output`")
                    .map_err(types::Error::trap)?;
                IsATTY::Yes
            } else {
                IsATTY::No
            },
        })?;

        for dir in host
            .get_directories()
            .context("failed to call `get-directories`")
            .map_err(types::Error::trap)?
        {
            descriptors.push(Descriptor::Directory {
                fd: dir.0,
                preopen_path: Some(dir.1),
            })?;
        }
        Ok(descriptors)
    }

    /// Returns next descriptor number, which was never assigned
    fn unused(&self) -> Result<u32> {
        match self.last_key_value() {
            Some((fd, _)) => {
                if let Some(fd) = fd.checked_add(1) {
                    return Ok(fd);
                }
                if self.len() == u32::MAX as usize {
                    return Err(types::Errno::Loop.into());
                }
                // TODO: Optimize
                Ok((0..u32::MAX)
                    .rev()
                    .find(|fd| !self.contains_key(fd))
                    .expect("failed to find an unused file descriptor"))
            }
            None => Ok(0),
        }
    }

    /// Removes the [Descriptor] corresponding to `fd`
    fn remove(&mut self, fd: types::Fd) -> Option<Descriptor> {
        let fd = fd.into();
        let desc = self.used.remove(&fd)?;
        self.free.push(fd);
        Some(desc)
    }

    /// Pushes the [Descriptor] returning corresponding number.
    /// This operation will try to reuse numbers previously removed via [`Self::remove`]
    /// and rely on [`Self::unused`] if no free numbers are recorded
    fn push(&mut self, desc: Descriptor) -> Result<u32> {
        let fd = if let Some(fd) = self.free.pop() {
            fd
        } else {
            self.unused()?
        };
        assert!(self.insert(fd, desc).is_none());
        Ok(fd)
    }
}

impl WasiPreview1Adapter {
    pub fn new() -> Self {
        Self::default()
    }
}

// Any context that needs to support preview 1 will impl this trait. They can
// construct the needed member with WasiPreview1Adapter::new().
pub trait WasiPreview1View: WasiView {
    fn adapter(&self) -> &WasiPreview1Adapter;
    fn adapter_mut(&mut self) -> &mut WasiPreview1Adapter;
}

/// A mutably-borrowed [`WasiPreview1View`] implementation, which provides access to the stored
/// state. It can be thought of as an in-flight [`WasiPreview1Adapter`] transaction, all
/// changes will be recorded in the underlying [`WasiPreview1Adapter`] returned by
/// [`WasiPreview1View::adapter_mut`] on [`Drop`] of this struct.
// NOTE: This exists for the most part just due to the fact that `bindgen` generates methods with
// `&mut self` receivers and so this struct lets us extend the lifetime of the `&mut self` borrow
// of the [`WasiPreview1View`] to provide means to return mutably and immutably borrowed [`Descriptors`]
// without having to rely on something like `Arc<Mutex<Descriptors>>`, while also being able to
// call methods like [`Descriptor::is_file`] and hiding complexity from preview1 method implementations.
struct Transaction<'a, T: WasiPreview1View + ?Sized> {
    view: &'a mut T,
    descriptors: Descriptors,
}

impl<T: WasiPreview1View + ?Sized> Drop for Transaction<'_, T> {
    /// Record changes in the [`WasiPreview1Adapter`] returned by [`WasiPreview1View::adapter_mut`]
    fn drop(&mut self) {
        let descriptors = mem::take(&mut self.descriptors);
        self.view.adapter_mut().descriptors = Some(descriptors);
    }
}

impl<T: WasiPreview1View + ?Sized> Transaction<'_, T> {
    /// Borrows [`Descriptor`] corresponding to `fd`.
    ///
    /// # Errors
    ///
    /// Returns [`types::Errno::Badf`] if no [`Descriptor`] is found
    fn get_descriptor(&self, fd: types::Fd) -> Result<&Descriptor> {
        let fd = fd.into();
        let desc = self.descriptors.get(&fd).ok_or(types::Errno::Badf)?;
        Ok(desc)
    }

    /// Borrows [`File`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`]
    fn get_file(&self, fd: types::Fd) -> Result<&File> {
        let fd = fd.into();
        match self.descriptors.get(&fd) {
            Some(Descriptor::File(file)) => Ok(file),
            _ => Err(types::Errno::Badf.into()),
        }
    }

    /// Mutably borrows [`File`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`]
    fn get_file_mut(&mut self, fd: types::Fd) -> Result<&mut File> {
        let fd = fd.into();
        match self.descriptors.get_mut(&fd) {
            Some(Descriptor::File(file)) => Ok(file),
            _ => Err(types::Errno::Badf.into()),
        }
    }

    /// Borrows [`File`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`]
    ///
    /// # Errors
    ///
    /// Returns [`types::Errno::Spipe`] if the descriptor corresponds to stdio
    fn get_seekable(&self, fd: types::Fd) -> Result<&File> {
        let fd = fd.into();
        match self.descriptors.get(&fd) {
            Some(Descriptor::File(file)) => Ok(file),
            Some(
                Descriptor::Stdin { .. } | Descriptor::Stdout { .. } | Descriptor::Stderr { .. },
            ) => {
                // NOTE: legacy implementation returns SPIPE here
                Err(types::Errno::Spipe.into())
            }
            _ => Err(types::Errno::Badf.into()),
        }
    }

    /// Returns [`filesystem::Descriptor`] corresponding to `fd`
    fn get_fd(&self, fd: types::Fd) -> Result<Resource<filesystem::Descriptor>> {
        match self.get_descriptor(fd)? {
            Descriptor::File(File { fd, .. }) => Ok(fd.borrowed()),
            Descriptor::Directory { fd, .. } => Ok(fd.borrowed()),
            Descriptor::Stdin { .. } | Descriptor::Stdout { .. } | Descriptor::Stderr { .. } => {
                Err(types::Errno::Badf.into())
            }
        }
    }

    /// Returns [`filesystem::Descriptor`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`]
    fn get_file_fd(&self, fd: types::Fd) -> Result<Resource<filesystem::Descriptor>> {
        self.get_file(fd).map(|File { fd, .. }| fd.borrowed())
    }

    /// Returns [`filesystem::Descriptor`] corresponding to `fd`
    /// if it describes a [`Descriptor::Directory`]
    fn get_dir_fd(&self, fd: types::Fd) -> Result<Resource<filesystem::Descriptor>> {
        let fd = fd.into();
        match self.descriptors.get(&fd) {
            Some(Descriptor::Directory { fd, .. }) => Ok(fd.borrowed()),
            _ => Err(types::Errno::Badf.into()),
        }
    }
}

trait WasiPreview1ViewExt:
    WasiPreview1View
    + preopens::Host
    + stdin::Host
    + stdout::Host
    + stderr::Host
    + terminal_input::Host
    + terminal_output::Host
    + terminal_stdin::Host
    + terminal_stdout::Host
    + terminal_stderr::Host
{
    /// Lazily initializes [`WasiPreview1Adapter`] returned by [`WasiPreview1View::adapter_mut`]
    /// and returns [`Transaction`] on success
    fn transact(&mut self) -> Result<Transaction<'_, Self>, types::Error> {
        let descriptors = if let Some(descriptors) = self.adapter_mut().descriptors.take() {
            descriptors
        } else {
            Descriptors::new(self)?
        }
        .into();
        Ok(Transaction {
            view: self,
            descriptors,
        })
    }

    /// Lazily initializes [`WasiPreview1Adapter`] returned by [`WasiPreview1View::adapter_mut`]
    /// and returns [`filesystem::Descriptor`] corresponding to `fd`
    fn get_fd(&mut self, fd: types::Fd) -> Result<Resource<filesystem::Descriptor>, types::Error> {
        let st = self.transact()?;
        let fd = st.get_fd(fd)?;
        Ok(fd)
    }

    /// Lazily initializes [`WasiPreview1Adapter`] returned by [`WasiPreview1View::adapter_mut`]
    /// and returns [`filesystem::Descriptor`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`] of [`crate::preview2::filesystem::File`] type
    fn get_file_fd(
        &mut self,
        fd: types::Fd,
    ) -> Result<Resource<filesystem::Descriptor>, types::Error> {
        let st = self.transact()?;
        let fd = st.get_file_fd(fd)?;
        Ok(fd)
    }

    /// Lazily initializes [`WasiPreview1Adapter`] returned by [`WasiPreview1View::adapter_mut`]
    /// and returns [`filesystem::Descriptor`] corresponding to `fd`
    /// if it describes a [`Descriptor::File`] or [`Descriptor::PreopenDirectory`]
    /// of [`crate::preview2::filesystem::Dir`] type
    fn get_dir_fd(
        &mut self,
        fd: types::Fd,
    ) -> Result<Resource<filesystem::Descriptor>, types::Error> {
        let st = self.transact()?;
        let fd = st.get_dir_fd(fd)?;
        Ok(fd)
    }
}

impl<T: WasiPreview1View + preopens::Host> WasiPreview1ViewExt for T {}

pub fn add_to_linker_async<T: WasiPreview1View>(
    linker: &mut wasmtime::Linker<T>,
) -> anyhow::Result<()> {
    wasi_snapshot_preview1::add_to_linker(linker, |t| t)
}

pub fn add_to_linker_sync<T: WasiPreview1View>(
    linker: &mut wasmtime::Linker<T>,
) -> anyhow::Result<()> {
    sync::add_wasi_snapshot_preview1_to_linker(linker, |t| t)
}

// Generate the wasi_snapshot_preview1::WasiSnapshotPreview1 trait,
// and the module types.
// None of the generated modules, traits, or types should be used externally
// to this module.
wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/witx/preview1/wasi_snapshot_preview1.witx"],
    async: {
        wasi_snapshot_preview1::{
            fd_advise, fd_close, fd_datasync, fd_fdstat_get, fd_filestat_get, fd_filestat_set_size,
            fd_filestat_set_times, fd_read, fd_pread, fd_seek, fd_sync, fd_readdir, fd_write,
            fd_pwrite, poll_oneoff, path_create_directory, path_filestat_get,
            path_filestat_set_times, path_link, path_open, path_readlink, path_remove_directory,
            path_rename, path_symlink, path_unlink_file
        }
    },
    errors: { errno => trappable Error },
});

mod sync {
    use anyhow::Result;
    use std::future::Future;

    wiggle::wasmtime_integration!({
        witx: ["$CARGO_MANIFEST_DIR/witx/preview1/wasi_snapshot_preview1.witx"],
        target: super,
        block_on[in_tokio]: {
            wasi_snapshot_preview1::{
                fd_advise, fd_close, fd_datasync, fd_fdstat_get, fd_filestat_get, fd_filestat_set_size,
                fd_filestat_set_times, fd_read, fd_pread, fd_seek, fd_sync, fd_readdir, fd_write,
                fd_pwrite, poll_oneoff, path_create_directory, path_filestat_get,
                path_filestat_set_times, path_link, path_open, path_readlink, path_remove_directory,
                path_rename, path_symlink, path_unlink_file
            }
        },
        errors: { errno => trappable Error },
    });

    // Small wrapper around `in_tokio` to add a `Result` layer which is always
    // `Ok`
    fn in_tokio<F: Future>(future: F) -> Result<F::Output> {
        Ok(crate::preview2::in_tokio(future))
    }
}

impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

impl From<StreamError> for types::Error {
    fn from(err: StreamError) -> Self {
        match err {
            StreamError::Closed => types::Errno::Io.into(),
            StreamError::LastOperationFailed(e) => match e.downcast::<std::io::Error>() {
                Ok(err) => filesystem::ErrorCode::from(err).into(),
                Err(e) => {
                    log::debug!("dropping error {e:?}");
                    types::Errno::Io.into()
                }
            },
            StreamError::Trap(e) => types::Error::trap(e),
        }
    }
}

impl From<FsError> for types::Error {
    fn from(err: FsError) -> Self {
        match err.downcast() {
            Ok(code) => code.into(),
            Err(e) => types::Error::trap(e),
        }
    }
}

fn systimespec(set: bool, ts: types::Timestamp, now: bool) -> Result<filesystem::NewTimestamp> {
    if set && now {
        Err(types::Errno::Inval.into())
    } else if set {
        Ok(filesystem::NewTimestamp::Timestamp(filesystem::Datetime {
            seconds: ts / 1_000_000_000,
            nanoseconds: (ts % 1_000_000_000) as _,
        }))
    } else if now {
        Ok(filesystem::NewTimestamp::Now)
    } else {
        Ok(filesystem::NewTimestamp::NoChange)
    }
}

impl TryFrom<wall_clock::Datetime> for types::Timestamp {
    type Error = types::Errno;

    fn try_from(
        wall_clock::Datetime {
            seconds,
            nanoseconds,
        }: wall_clock::Datetime,
    ) -> Result<Self, Self::Error> {
        types::Timestamp::from(seconds)
            .checked_mul(1_000_000_000)
            .and_then(|ns| ns.checked_add(nanoseconds.into()))
            .ok_or(types::Errno::Overflow)
    }
}

impl From<types::Lookupflags> for filesystem::PathFlags {
    fn from(flags: types::Lookupflags) -> Self {
        if flags.contains(types::Lookupflags::SYMLINK_FOLLOW) {
            filesystem::PathFlags::SYMLINK_FOLLOW
        } else {
            filesystem::PathFlags::empty()
        }
    }
}

impl From<types::Oflags> for filesystem::OpenFlags {
    fn from(flags: types::Oflags) -> Self {
        let mut out = filesystem::OpenFlags::empty();
        if flags.contains(types::Oflags::CREAT) {
            out |= filesystem::OpenFlags::CREATE;
        }
        if flags.contains(types::Oflags::DIRECTORY) {
            out |= filesystem::OpenFlags::DIRECTORY;
        }
        if flags.contains(types::Oflags::EXCL) {
            out |= filesystem::OpenFlags::EXCLUSIVE;
        }
        if flags.contains(types::Oflags::TRUNC) {
            out |= filesystem::OpenFlags::TRUNCATE;
        }
        out
    }
}

impl From<types::Advice> for filesystem::Advice {
    fn from(advice: types::Advice) -> Self {
        match advice {
            types::Advice::Normal => filesystem::Advice::Normal,
            types::Advice::Sequential => filesystem::Advice::Sequential,
            types::Advice::Random => filesystem::Advice::Random,
            types::Advice::Willneed => filesystem::Advice::WillNeed,
            types::Advice::Dontneed => filesystem::Advice::DontNeed,
            types::Advice::Noreuse => filesystem::Advice::NoReuse,
        }
    }
}

impl TryFrom<filesystem::DescriptorType> for types::Filetype {
    type Error = anyhow::Error;

    fn try_from(ty: filesystem::DescriptorType) -> Result<Self, Self::Error> {
        match ty {
            filesystem::DescriptorType::RegularFile => Ok(types::Filetype::RegularFile),
            filesystem::DescriptorType::Directory => Ok(types::Filetype::Directory),
            filesystem::DescriptorType::BlockDevice => Ok(types::Filetype::BlockDevice),
            filesystem::DescriptorType::CharacterDevice => Ok(types::Filetype::CharacterDevice),
            // preview1 never had a FIFO code.
            filesystem::DescriptorType::Fifo => Ok(types::Filetype::Unknown),
            // TODO: Add a way to disginguish between FILETYPE_SOCKET_STREAM and
            // FILETYPE_SOCKET_DGRAM.
            filesystem::DescriptorType::Socket => {
                bail!("sockets are not currently supported")
            }
            filesystem::DescriptorType::SymbolicLink => Ok(types::Filetype::SymbolicLink),
            filesystem::DescriptorType::Unknown => Ok(types::Filetype::Unknown),
        }
    }
}

impl From<IsATTY> for types::Filetype {
    fn from(isatty: IsATTY) -> Self {
        match isatty {
            IsATTY::Yes => types::Filetype::CharacterDevice,
            IsATTY::No => types::Filetype::Unknown,
        }
    }
}

impl From<filesystem::ErrorCode> for types::Errno {
    fn from(code: filesystem::ErrorCode) -> Self {
        match code {
            filesystem::ErrorCode::Access => types::Errno::Acces,
            filesystem::ErrorCode::WouldBlock => types::Errno::Again,
            filesystem::ErrorCode::Already => types::Errno::Already,
            filesystem::ErrorCode::BadDescriptor => types::Errno::Badf,
            filesystem::ErrorCode::Busy => types::Errno::Busy,
            filesystem::ErrorCode::Deadlock => types::Errno::Deadlk,
            filesystem::ErrorCode::Quota => types::Errno::Dquot,
            filesystem::ErrorCode::Exist => types::Errno::Exist,
            filesystem::ErrorCode::FileTooLarge => types::Errno::Fbig,
            filesystem::ErrorCode::IllegalByteSequence => types::Errno::Ilseq,
            filesystem::ErrorCode::InProgress => types::Errno::Inprogress,
            filesystem::ErrorCode::Interrupted => types::Errno::Intr,
            filesystem::ErrorCode::Invalid => types::Errno::Inval,
            filesystem::ErrorCode::Io => types::Errno::Io,
            filesystem::ErrorCode::IsDirectory => types::Errno::Isdir,
            filesystem::ErrorCode::Loop => types::Errno::Loop,
            filesystem::ErrorCode::TooManyLinks => types::Errno::Mlink,
            filesystem::ErrorCode::MessageSize => types::Errno::Msgsize,
            filesystem::ErrorCode::NameTooLong => types::Errno::Nametoolong,
            filesystem::ErrorCode::NoDevice => types::Errno::Nodev,
            filesystem::ErrorCode::NoEntry => types::Errno::Noent,
            filesystem::ErrorCode::NoLock => types::Errno::Nolck,
            filesystem::ErrorCode::InsufficientMemory => types::Errno::Nomem,
            filesystem::ErrorCode::InsufficientSpace => types::Errno::Nospc,
            filesystem::ErrorCode::Unsupported => types::Errno::Notsup,
            filesystem::ErrorCode::NotDirectory => types::Errno::Notdir,
            filesystem::ErrorCode::NotEmpty => types::Errno::Notempty,
            filesystem::ErrorCode::NotRecoverable => types::Errno::Notrecoverable,
            filesystem::ErrorCode::NoTty => types::Errno::Notty,
            filesystem::ErrorCode::NoSuchDevice => types::Errno::Nxio,
            filesystem::ErrorCode::Overflow => types::Errno::Overflow,
            filesystem::ErrorCode::NotPermitted => types::Errno::Perm,
            filesystem::ErrorCode::Pipe => types::Errno::Pipe,
            filesystem::ErrorCode::ReadOnly => types::Errno::Rofs,
            filesystem::ErrorCode::InvalidSeek => types::Errno::Spipe,
            filesystem::ErrorCode::TextFileBusy => types::Errno::Txtbsy,
            filesystem::ErrorCode::CrossDevice => types::Errno::Xdev,
        }
    }
}

impl From<std::num::TryFromIntError> for types::Error {
    fn from(_: std::num::TryFromIntError) -> Self {
        types::Errno::Overflow.into()
    }
}

impl From<GuestError> for types::Error {
    fn from(err: GuestError) -> Self {
        use wiggle::GuestError::*;
        match err {
            InvalidFlagValue { .. } => types::Errno::Inval.into(),
            InvalidEnumValue { .. } => types::Errno::Inval.into(),
            // As per
            // https://github.com/WebAssembly/wasi/blob/main/legacy/tools/witx-docs.md#pointers
            //
            // > If a misaligned pointer is passed to a function, the function
            // > shall trap.
            // >
            // > If an out-of-bounds pointer is passed to a function and the
            // > function needs to dereference it, the function shall trap.
            //
            // so this turns OOB and misalignment errors into traps.
            PtrOverflow { .. } | PtrOutOfBounds { .. } | PtrNotAligned { .. } => {
                types::Error::trap(err.into())
            }
            PtrBorrowed { .. } => types::Errno::Fault.into(),
            InvalidUtf8 { .. } => types::Errno::Ilseq.into(),
            TryFromIntError { .. } => types::Errno::Overflow.into(),
            SliceLengthsDiffer { .. } => types::Errno::Fault.into(),
            BorrowCheckerOutOfHandles { .. } => types::Errno::Fault.into(),
            InFunc { err, .. } => types::Error::from(*err),
        }
    }
}

impl From<filesystem::ErrorCode> for types::Error {
    fn from(code: filesystem::ErrorCode) -> Self {
        types::Errno::from(code).into()
    }
}

impl From<wasmtime::component::ResourceTableError> for types::Error {
    fn from(err: wasmtime::component::ResourceTableError) -> Self {
        types::Error::trap(err.into())
    }
}

type Result<T, E = types::Error> = std::result::Result<T, E>;

fn write_bytes<'a>(
    ptr: impl Borrow<GuestPtr<'a, u8>>,
    buf: &[u8],
) -> Result<GuestPtr<'a, u8>, types::Error> {
    // NOTE: legacy implementation always returns Inval errno

    let buf = buf.as_ref();
    let len = buf.len().try_into()?;

    let ptr = ptr.borrow();
    ptr.as_array(len).copy_from_slice(buf)?;
    let next = ptr.add(len)?;
    Ok(next)
}

fn write_byte<'a>(ptr: impl Borrow<GuestPtr<'a, u8>>, byte: u8) -> Result<GuestPtr<'a, u8>> {
    let ptr = ptr.borrow();
    ptr.write(byte)?;
    let next = ptr.add(1)?;
    Ok(next)
}

fn read_str<'a>(ptr: impl Borrow<GuestPtr<'a, str>>) -> Result<GuestStrCow<'a>> {
    let s = ptr.borrow().as_cow()?;
    Ok(s)
}

fn read_string<'a>(ptr: impl Borrow<GuestPtr<'a, str>>) -> Result<String> {
    read_str(ptr).map(|s| s.to_string())
}

// Find first non-empty buffer.
fn first_non_empty_ciovec<'a, 'b>(
    ciovs: &'a types::CiovecArray<'b>,
) -> Result<Option<GuestPtr<'a, [u8]>>> {
    for iov in ciovs.iter() {
        let iov = iov?.read()?;
        if iov.buf_len == 0 {
            continue;
        }
        return Ok(Some(iov.buf.as_array(iov.buf_len)));
    }
    Ok(None)
}

// Find first non-empty buffer.
fn first_non_empty_iovec<'a>(iovs: &types::IovecArray<'a>) -> Result<Option<GuestPtr<'a, [u8]>>> {
    for iov in iovs.iter() {
        let iov = iov?.read()?;
        if iov.buf_len == 0 {
            continue;
        }
        return Ok(Some(iov.buf.as_array(iov.buf_len)));
    }
    Ok(None)
}

#[async_trait::async_trait]
// Implement the WasiSnapshotPreview1 trait using only the traits that are
// required for T, i.e., in terms of the preview 2 wit interface, and state
// stored in the WasiPreview1Adapter struct.
impl<
        T: WasiPreview1View
            + bindings::cli::environment::Host
            + bindings::cli::exit::Host
            + bindings::filesystem::preopens::Host
            + bindings::filesystem::types::Host
            + bindings::io::poll::Host
            + bindings::random::random::Host
            + bindings::io::streams::Host
            + bindings::clocks::monotonic_clock::Host
            + bindings::clocks::wall_clock::Host,
    > wasi_snapshot_preview1::WasiSnapshotPreview1 for T
{
    #[instrument(skip(self))]
    fn args_get<'b>(
        &mut self,
        argv: &GuestPtr<'b, GuestPtr<'b, u8>>,
        argv_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), types::Error> {
        self.get_arguments()
            .context("failed to call `get-arguments`")
            .map_err(types::Error::trap)?
            .into_iter()
            .try_fold((*argv, *argv_buf), |(argv, argv_buf), arg| -> Result<_> {
                argv.write(argv_buf)?;
                let argv = argv.add(1)?;

                let argv_buf = write_bytes(argv_buf, arg.as_bytes())?;
                let argv_buf = write_byte(argv_buf, 0)?;

                Ok((argv, argv_buf))
            })?;
        Ok(())
    }

    #[instrument(skip(self))]
    fn args_sizes_get(&mut self) -> Result<(types::Size, types::Size), types::Error> {
        let args = self
            .get_arguments()
            .context("failed to call `get-arguments`")
            .map_err(types::Error::trap)?;
        let num = args.len().try_into().map_err(|_| types::Errno::Overflow)?;
        let len = args
            .iter()
            .map(|buf| buf.len() + 1) // Each argument is expected to be `\0` terminated.
            .sum::<usize>()
            .try_into()
            .map_err(|_| types::Errno::Overflow)?;
        Ok((num, len))
    }

    #[instrument(skip(self))]
    fn environ_get<'b>(
        &mut self,
        environ: &GuestPtr<'b, GuestPtr<'b, u8>>,
        environ_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), types::Error> {
        self.get_environment()
            .context("failed to call `get-environment`")
            .map_err(types::Error::trap)?
            .into_iter()
            .try_fold(
                (*environ, *environ_buf),
                |(environ, environ_buf), (k, v)| -> Result<_, types::Error> {
                    environ.write(environ_buf)?;
                    let environ = environ.add(1)?;

                    let environ_buf = write_bytes(environ_buf, k.as_bytes())?;
                    let environ_buf = write_byte(environ_buf, b'=')?;
                    let environ_buf = write_bytes(environ_buf, v.as_bytes())?;
                    let environ_buf = write_byte(environ_buf, 0)?;

                    Ok((environ, environ_buf))
                },
            )?;
        Ok(())
    }

    #[instrument(skip(self))]
    fn environ_sizes_get(&mut self) -> Result<(types::Size, types::Size), types::Error> {
        let environ = self
            .get_environment()
            .context("failed to call `get-environment`")
            .map_err(types::Error::trap)?;
        let num = environ.len().try_into()?;
        let len = environ
            .iter()
            .map(|(k, v)| k.len() + 1 + v.len() + 1) // Key/value pairs are expected to be joined with `=`s, and terminated with `\0`s.
            .sum::<usize>()
            .try_into()?;
        Ok((num, len))
    }

    #[instrument(skip(self))]
    fn clock_res_get(&mut self, id: types::Clockid) -> Result<types::Timestamp, types::Error> {
        let res = match id {
            types::Clockid::Realtime => wall_clock::Host::resolution(self)
                .context("failed to call `wall_clock::resolution`")
                .map_err(types::Error::trap)?
                .try_into()?,
            types::Clockid::Monotonic => monotonic_clock::Host::resolution(self)
                .context("failed to call `monotonic_clock::resolution`")
                .map_err(types::Error::trap)?,
            types::Clockid::ProcessCputimeId | types::Clockid::ThreadCputimeId => {
                return Err(types::Errno::Badf.into())
            }
        };
        Ok(res)
    }

    #[instrument(skip(self))]
    fn clock_time_get(
        &mut self,
        id: types::Clockid,
        _precision: types::Timestamp,
    ) -> Result<types::Timestamp, types::Error> {
        let now = match id {
            types::Clockid::Realtime => wall_clock::Host::now(self)
                .context("failed to call `wall_clock::now`")
                .map_err(types::Error::trap)?
                .try_into()?,
            types::Clockid::Monotonic => monotonic_clock::Host::now(self)
                .context("failed to call `monotonic_clock::now`")
                .map_err(types::Error::trap)?,
            types::Clockid::ProcessCputimeId | types::Clockid::ThreadCputimeId => {
                return Err(types::Errno::Badf.into())
            }
        };
        Ok(now)
    }

    #[instrument(skip(self))]
    async fn fd_advise(
        &mut self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), types::Error> {
        let fd = self.get_file_fd(fd)?;
        self.advise(fd, offset, len, advice.into())
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `advise`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    /// Force the allocation of space in a file.
    /// NOTE: This is similar to `posix_fallocate` in POSIX.
    #[instrument(skip(self))]
    fn fd_allocate(
        &mut self,
        fd: types::Fd,
        _offset: types::Filesize,
        _len: types::Filesize,
    ) -> Result<(), types::Error> {
        self.get_file_fd(fd)?;
        Err(types::Errno::Notsup.into())
    }

    /// Close a file descriptor.
    /// NOTE: This is similar to `close` in POSIX.
    #[instrument(skip(self))]
    async fn fd_close(&mut self, fd: types::Fd) -> Result<(), types::Error> {
        let desc = self
            .transact()?
            .descriptors
            .remove(fd)
            .ok_or(types::Errno::Badf)?;
        match desc {
            Descriptor::Stdin { stream, .. } => streams::HostInputStream::drop(self, stream)
                .context("failed to call `drop` on `input-stream`"),
            Descriptor::Stdout { stream, .. } | Descriptor::Stderr { stream, .. } => {
                streams::HostOutputStream::drop(self, stream)
                    .context("failed to call `drop` on `output-stream`")
            }
            Descriptor::File(File { fd, .. }) | Descriptor::Directory { fd, .. } => {
                filesystem::HostDescriptor::drop(self, fd).context("failed to call `drop`")
            }
        }
        .map_err(types::Error::trap)
    }

    /// Synchronize the data of a file to disk.
    /// NOTE: This is similar to `fdatasync` in POSIX.
    #[instrument(skip(self))]
    async fn fd_datasync(&mut self, fd: types::Fd) -> Result<(), types::Error> {
        let fd = self.get_file_fd(fd)?;
        self.sync_data(fd).await.map_err(|e| {
            e.try_into()
                .context("failed to call `sync-data`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Get the attributes of a file descriptor.
    /// NOTE: This returns similar flags to `fsync(fd, F_GETFL)` in POSIX, as well as additional fields.
    #[instrument(skip(self))]
    async fn fd_fdstat_get(&mut self, fd: types::Fd) -> Result<types::Fdstat, types::Error> {
        let (fd, blocking, append) = match self.transact()?.get_descriptor(fd)? {
            Descriptor::Stdin { isatty, .. } => {
                let fs_rights_base = types::Rights::FD_READ;
                return Ok(types::Fdstat {
                    fs_filetype: (*isatty).into(),
                    fs_flags: types::Fdflags::empty(),
                    fs_rights_base,
                    fs_rights_inheriting: fs_rights_base,
                });
            }
            Descriptor::Stdout { isatty, .. } | Descriptor::Stderr { isatty, .. } => {
                let fs_rights_base = types::Rights::FD_WRITE;
                return Ok(types::Fdstat {
                    fs_filetype: (*isatty).into(),
                    fs_flags: types::Fdflags::empty(),
                    fs_rights_base,
                    fs_rights_inheriting: fs_rights_base,
                });
            }
            Descriptor::Directory {
                preopen_path: Some(_),
                ..
            } => {
                // Hard-coded set or rights expected by many userlands:
                let fs_rights_base = types::Rights::PATH_CREATE_DIRECTORY
                    | types::Rights::PATH_CREATE_FILE
                    | types::Rights::PATH_LINK_SOURCE
                    | types::Rights::PATH_LINK_TARGET
                    | types::Rights::PATH_OPEN
                    | types::Rights::FD_READDIR
                    | types::Rights::PATH_READLINK
                    | types::Rights::PATH_RENAME_SOURCE
                    | types::Rights::PATH_RENAME_TARGET
                    | types::Rights::PATH_SYMLINK
                    | types::Rights::PATH_REMOVE_DIRECTORY
                    | types::Rights::PATH_UNLINK_FILE
                    | types::Rights::PATH_FILESTAT_GET
                    | types::Rights::PATH_FILESTAT_SET_TIMES
                    | types::Rights::FD_FILESTAT_GET
                    | types::Rights::FD_FILESTAT_SET_TIMES;

                let fs_rights_inheriting = fs_rights_base
                    | types::Rights::FD_DATASYNC
                    | types::Rights::FD_READ
                    | types::Rights::FD_SEEK
                    | types::Rights::FD_FDSTAT_SET_FLAGS
                    | types::Rights::FD_SYNC
                    | types::Rights::FD_TELL
                    | types::Rights::FD_WRITE
                    | types::Rights::FD_ADVISE
                    | types::Rights::FD_ALLOCATE
                    | types::Rights::FD_FILESTAT_GET
                    | types::Rights::FD_FILESTAT_SET_SIZE
                    | types::Rights::FD_FILESTAT_SET_TIMES
                    | types::Rights::POLL_FD_READWRITE;

                return Ok(types::Fdstat {
                    fs_filetype: types::Filetype::Directory,
                    fs_flags: types::Fdflags::empty(),
                    fs_rights_base,
                    fs_rights_inheriting,
                });
            }
            Descriptor::Directory { fd, .. } => (fd.borrowed(), BlockingMode::Blocking, false),
            Descriptor::File(File {
                fd,
                blocking_mode,
                append,
                ..
            }) => (fd.borrowed(), *blocking_mode, *append),
        };
        let flags = self.get_flags(fd.borrowed()).await.map_err(|e| {
            e.try_into()
                .context("failed to call `get-flags`")
                .unwrap_or_else(types::Error::trap)
        })?;
        let fs_filetype = self
            .get_type(fd.borrowed())
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `get-type`")
                    .unwrap_or_else(types::Error::trap)
            })?
            .try_into()
            .map_err(types::Error::trap)?;
        let mut fs_flags = types::Fdflags::empty();
        let mut fs_rights_base = types::Rights::all();
        if let types::Filetype::Directory = fs_filetype {
            fs_rights_base &= !types::Rights::FD_SEEK;
        }
        if !flags.contains(filesystem::DescriptorFlags::READ) {
            fs_rights_base &= !types::Rights::FD_READ;
        }
        if !flags.contains(filesystem::DescriptorFlags::WRITE) {
            fs_rights_base &= !types::Rights::FD_WRITE;
        }
        if flags.contains(filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC) {
            fs_flags |= types::Fdflags::DSYNC;
        }
        if flags.contains(filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC) {
            fs_flags |= types::Fdflags::RSYNC;
        }
        if flags.contains(filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC) {
            fs_flags |= types::Fdflags::SYNC;
        }
        if append {
            fs_flags |= types::Fdflags::APPEND;
        }
        if matches!(blocking, BlockingMode::NonBlocking) {
            fs_flags |= types::Fdflags::NONBLOCK;
        }
        Ok(types::Fdstat {
            fs_filetype,
            fs_flags,
            fs_rights_base,
            fs_rights_inheriting: fs_rights_base,
        })
    }

    /// Adjust the flags associated with a file descriptor.
    /// NOTE: This is similar to `fcntl(fd, F_SETFL, flags)` in POSIX.
    #[instrument(skip(self))]
    fn fd_fdstat_set_flags(
        &mut self,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<(), types::Error> {
        let mut st = self.transact()?;
        let File {
            append,
            blocking_mode,
            ..
        } = st.get_file_mut(fd)?;

        // Only support changing the NONBLOCK or APPEND flags.
        if flags.contains(types::Fdflags::DSYNC)
            || flags.contains(types::Fdflags::SYNC)
            || flags.contains(types::Fdflags::RSYNC)
        {
            return Err(types::Errno::Inval.into());
        }
        *append = flags.contains(types::Fdflags::APPEND);
        *blocking_mode = BlockingMode::from_fdflags(&flags);
        Ok(())
    }

    /// Does not do anything if `fd` corresponds to a valid descriptor and returns `[types::Errno::Badf]` error otherwise.
    #[instrument(skip(self))]
    fn fd_fdstat_set_rights(
        &mut self,
        fd: types::Fd,
        _fs_rights_base: types::Rights,
        _fs_rights_inheriting: types::Rights,
    ) -> Result<(), types::Error> {
        self.get_fd(fd)?;
        Ok(())
    }

    /// Return the attributes of an open file.
    #[instrument(skip(self))]
    async fn fd_filestat_get(&mut self, fd: types::Fd) -> Result<types::Filestat, types::Error> {
        let t = self.transact()?;
        let desc = t.get_descriptor(fd)?;
        match desc {
            Descriptor::Stdin { isatty, .. }
            | Descriptor::Stdout { isatty, .. }
            | Descriptor::Stderr { isatty, .. } => Ok(types::Filestat {
                dev: 0,
                ino: 0,
                filetype: (*isatty).into(),
                nlink: 0,
                size: 0,
                atim: 0,
                mtim: 0,
                ctim: 0,
            }),
            Descriptor::Directory { fd, .. } | Descriptor::File(File { fd, .. }) => {
                let fd = fd.borrowed();
                drop(t);
                let filesystem::DescriptorStat {
                    type_,
                    link_count: nlink,
                    size,
                    data_access_timestamp,
                    data_modification_timestamp,
                    status_change_timestamp,
                } = self.stat(fd.borrowed()).await.map_err(|e| {
                    e.try_into()
                        .context("failed to call `stat`")
                        .unwrap_or_else(types::Error::trap)
                })?;
                let metadata_hash = self.metadata_hash(fd).await.map_err(|e| {
                    e.try_into()
                        .context("failed to call `metadata_hash`")
                        .unwrap_or_else(types::Error::trap)
                })?;
                let filetype = type_.try_into().map_err(types::Error::trap)?;
                let zero = wall_clock::Datetime {
                    seconds: 0,
                    nanoseconds: 0,
                };
                let atim = data_access_timestamp.unwrap_or(zero).try_into()?;
                let mtim = data_modification_timestamp.unwrap_or(zero).try_into()?;
                let ctim = status_change_timestamp.unwrap_or(zero).try_into()?;
                Ok(types::Filestat {
                    dev: 1,
                    ino: metadata_hash.lower,
                    filetype,
                    nlink,
                    size,
                    atim,
                    mtim,
                    ctim,
                })
            }
        }
    }

    /// Adjust the size of an open file. If this increases the file's size, the extra bytes are filled with zeros.
    /// NOTE: This is similar to `ftruncate` in POSIX.
    #[instrument(skip(self))]
    async fn fd_filestat_set_size(
        &mut self,
        fd: types::Fd,
        size: types::Filesize,
    ) -> Result<(), types::Error> {
        let fd = self.get_file_fd(fd)?;
        self.set_size(fd, size).await.map_err(|e| {
            e.try_into()
                .context("failed to call `set-size`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Adjust the timestamps of an open file or directory.
    /// NOTE: This is similar to `futimens` in POSIX.
    #[instrument(skip(self))]
    async fn fd_filestat_set_times(
        &mut self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), types::Error> {
        let atim = systimespec(
            fst_flags.contains(types::Fstflags::ATIM),
            atim,
            fst_flags.contains(types::Fstflags::ATIM_NOW),
        )?;
        let mtim = systimespec(
            fst_flags.contains(types::Fstflags::MTIM),
            mtim,
            fst_flags.contains(types::Fstflags::MTIM_NOW),
        )?;

        let fd = self.get_fd(fd)?;
        self.set_times(fd, atim, mtim).await.map_err(|e| {
            e.try_into()
                .context("failed to call `set-times`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Read from a file descriptor.
    /// NOTE: This is similar to `readv` in POSIX.
    #[instrument(skip(self))]
    async fn fd_read<'a>(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'a>,
    ) -> Result<types::Size, types::Error> {
        let t = self.transact()?;
        let desc = t.get_descriptor(fd)?;
        let (buf, read) = match desc {
            Descriptor::File(File {
                fd,
                blocking_mode,
                position,
                ..
            }) => {
                let fd = fd.borrowed();
                let blocking_mode = *blocking_mode;
                let position = position.clone();
                drop(t);
                let Some(buf) = first_non_empty_iovec(iovs)? else {
                    return Ok(0);
                };

                let pos = position.load(Ordering::Relaxed);
                let stream = self.read_via_stream(fd.borrowed(), pos).map_err(|e| {
                    e.try_into()
                        .context("failed to call `read-via-stream`")
                        .unwrap_or_else(types::Error::trap)
                })?;
                let read = blocking_mode
                    .read(self, stream.borrowed(), buf.len().try_into()?)
                    .await;
                streams::HostInputStream::drop(self, stream).map_err(|e| types::Error::trap(e))?;
                let read = read?;
                let n = read.len().try_into()?;
                let pos = pos.checked_add(n).ok_or(types::Errno::Overflow)?;
                position.store(pos, Ordering::Relaxed);

                (buf, read)
            }
            Descriptor::Stdin { stream, .. } => {
                let stream = stream.borrowed();
                drop(t);
                let Some(buf) = first_non_empty_iovec(iovs)? else {
                    return Ok(0);
                };
                let read = BlockingMode::Blocking
                    .read(self, stream, buf.len().try_into()?)
                    .await?;
                (buf, read)
            }
            _ => return Err(types::Errno::Badf.into()),
        };
        if read.len() > buf.len().try_into()? {
            return Err(types::Errno::Range.into());
        }
        let buf = buf.get_range(0..u32::try_from(read.len())?).unwrap();
        buf.copy_from_slice(&read)?;
        let n = read.len().try_into()?;
        Ok(n)
    }

    /// Read from a file descriptor, without using and updating the file descriptor's offset.
    /// NOTE: This is similar to `preadv` in POSIX.
    #[instrument(skip(self))]
    async fn fd_pread<'a>(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'a>,
        offset: types::Filesize,
    ) -> Result<types::Size, types::Error> {
        let t = self.transact()?;
        let desc = t.get_descriptor(fd)?;
        let (buf, read) = match desc {
            Descriptor::File(File {
                fd, blocking_mode, ..
            }) => {
                let fd = fd.borrowed();
                let blocking_mode = *blocking_mode;
                drop(t);
                let Some(buf) = first_non_empty_iovec(iovs)? else {
                    return Ok(0);
                };

                let stream = self.read_via_stream(fd, offset).map_err(|e| {
                    e.try_into()
                        .context("failed to call `read-via-stream`")
                        .unwrap_or_else(types::Error::trap)
                })?;
                let read = blocking_mode
                    .read(self, stream.borrowed(), buf.len().try_into()?)
                    .await;
                streams::HostInputStream::drop(self, stream).map_err(|e| types::Error::trap(e))?;
                (buf, read?)
            }
            Descriptor::Stdin { .. } => {
                // NOTE: legacy implementation returns SPIPE here
                return Err(types::Errno::Spipe.into());
            }
            _ => return Err(types::Errno::Badf.into()),
        };
        if read.len() > buf.len().try_into()? {
            return Err(types::Errno::Range.into());
        }
        let buf = buf.get_range(0..u32::try_from(read.len())?).unwrap();
        buf.copy_from_slice(&read)?;
        let n = read.len().try_into()?;
        Ok(n)
    }

    /// Write to a file descriptor.
    /// NOTE: This is similar to `writev` in POSIX.
    #[instrument(skip(self))]
    async fn fd_write<'a>(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'a>,
    ) -> Result<types::Size, types::Error> {
        let t = self.transact()?;
        let desc = t.get_descriptor(fd)?;
        match desc {
            Descriptor::File(File {
                fd,
                blocking_mode,
                append,
                position,
            }) => {
                let fd = fd.borrowed();
                let fd2 = fd.borrowed();
                let blocking_mode = *blocking_mode;
                let position = position.clone();
                let append = *append;
                drop(t);
                let Some(buf) = first_non_empty_ciovec(ciovs)? else {
                    return Ok(0);
                };
                let (stream, pos) = if append {
                    let stream = self.append_via_stream(fd).map_err(|e| {
                        e.try_into()
                            .context("failed to call `append-via-stream`")
                            .unwrap_or_else(types::Error::trap)
                    })?;
                    (stream, 0)
                } else {
                    let pos = position.load(Ordering::Relaxed);
                    let stream = self.write_via_stream(fd, pos).map_err(|e| {
                        e.try_into()
                            .context("failed to call `write-via-stream`")
                            .unwrap_or_else(types::Error::trap)
                    })?;
                    (stream, pos)
                };
                let n = blocking_mode.write(self, stream.borrowed(), buf).await;
                streams::HostOutputStream::drop(self, stream).map_err(|e| types::Error::trap(e))?;
                let n = n?;
                if append {
                    let len = self.stat(fd2).await?;
                    position.store(len.size, Ordering::Relaxed);
                } else {
                    let pos = pos.checked_add(n as u64).ok_or(types::Errno::Overflow)?;
                    position.store(pos, Ordering::Relaxed);
                }
                let n = n.try_into()?;
                Ok(n)
            }
            Descriptor::Stdout { stream, .. } | Descriptor::Stderr { stream, .. } => {
                let stream = stream.borrowed();
                drop(t);
                let Some(buf) = first_non_empty_ciovec(ciovs)? else {
                    return Ok(0);
                };
                let n = BlockingMode::Blocking
                    .write(self, stream, buf)
                    .await?
                    .try_into()?;
                Ok(n)
            }
            _ => Err(types::Errno::Badf.into()),
        }
    }

    /// Write to a file descriptor, without using and updating the file descriptor's offset.
    /// NOTE: This is similar to `pwritev` in POSIX.
    #[instrument(skip(self))]
    async fn fd_pwrite<'a>(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'a>,
        offset: types::Filesize,
    ) -> Result<types::Size, types::Error> {
        let t = self.transact()?;
        let desc = t.get_descriptor(fd)?;
        let n = match desc {
            Descriptor::File(File {
                fd, blocking_mode, ..
            }) => {
                let fd = fd.borrowed();
                let blocking_mode = *blocking_mode;
                drop(t);
                let Some(buf) = first_non_empty_ciovec(ciovs)? else {
                    return Ok(0);
                };
                let stream = self.write_via_stream(fd, offset).map_err(|e| {
                    e.try_into()
                        .context("failed to call `write-via-stream`")
                        .unwrap_or_else(types::Error::trap)
                })?;
                let result = blocking_mode.write(self, stream.borrowed(), buf).await;
                streams::HostOutputStream::drop(self, stream).map_err(|e| types::Error::trap(e))?;
                result?
            }
            Descriptor::Stdout { .. } | Descriptor::Stderr { .. } => {
                // NOTE: legacy implementation returns SPIPE here
                return Err(types::Errno::Spipe.into());
            }
            _ => return Err(types::Errno::Badf.into()),
        };
        Ok(n.try_into()?)
    }

    /// Return a description of the given preopened file descriptor.
    #[instrument(skip(self))]
    fn fd_prestat_get(&mut self, fd: types::Fd) -> Result<types::Prestat, types::Error> {
        if let Descriptor::Directory {
            preopen_path: Some(p),
            ..
        } = self.transact()?.get_descriptor(fd)?
        {
            let pr_name_len = p.len().try_into()?;
            return Ok(types::Prestat::Dir(types::PrestatDir { pr_name_len }));
        }
        Err(types::Errno::Badf.into()) // NOTE: legacy implementation returns BADF here
    }

    /// Return a description of the given preopened file descriptor.
    #[instrument(skip(self))]
    fn fd_prestat_dir_name<'a>(
        &mut self,
        fd: types::Fd,
        path: &GuestPtr<'a, u8>,
        path_max_len: types::Size,
    ) -> Result<(), types::Error> {
        let path_max_len = path_max_len.try_into()?;
        if let Descriptor::Directory {
            preopen_path: Some(p),
            ..
        } = self.transact()?.get_descriptor(fd)?
        {
            if p.len() > path_max_len {
                return Err(types::Errno::Nametoolong.into());
            }
            write_bytes(path, p.as_bytes())?;
            return Ok(());
        }
        Err(types::Errno::Notdir.into()) // NOTE: legacy implementation returns NOTDIR here
    }

    /// Atomically replace a file descriptor by renumbering another file descriptor.
    #[instrument(skip(self))]
    fn fd_renumber(&mut self, from: types::Fd, to: types::Fd) -> Result<(), types::Error> {
        let mut st = self.transact()?;
        let desc = st.descriptors.remove(from).ok_or(types::Errno::Badf)?;
        st.descriptors.insert(to.into(), desc);
        Ok(())
    }

    /// Move the offset of a file descriptor.
    /// NOTE: This is similar to `lseek` in POSIX.
    #[instrument(skip(self))]
    async fn fd_seek(
        &mut self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, types::Error> {
        let t = self.transact()?;
        let File { fd, position, .. } = t.get_seekable(fd)?;
        let fd = fd.borrowed();
        let position = position.clone();
        drop(t);
        let pos = match whence {
            types::Whence::Set if offset >= 0 => {
                offset.try_into().map_err(|_| types::Errno::Inval)?
            }
            types::Whence::Cur => position
                .load(Ordering::Relaxed)
                .checked_add_signed(offset)
                .ok_or(types::Errno::Inval)?,
            types::Whence::End => {
                let filesystem::DescriptorStat { size, .. } = self.stat(fd).await.map_err(|e| {
                    e.try_into()
                        .context("failed to call `stat`")
                        .unwrap_or_else(types::Error::trap)
                })?;
                size.checked_add_signed(offset).ok_or(types::Errno::Inval)?
            }
            _ => return Err(types::Errno::Inval.into()),
        };
        position.store(pos, Ordering::Relaxed);
        Ok(pos)
    }

    /// Synchronize the data and metadata of a file to disk.
    /// NOTE: This is similar to `fsync` in POSIX.
    #[instrument(skip(self))]
    async fn fd_sync(&mut self, fd: types::Fd) -> Result<(), types::Error> {
        let fd = self.get_file_fd(fd)?;
        self.sync(fd).await.map_err(|e| {
            e.try_into()
                .context("failed to call `sync`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Return the current offset of a file descriptor.
    /// NOTE: This is similar to `lseek(fd, 0, SEEK_CUR)` in POSIX.
    #[instrument(skip(self))]
    fn fd_tell(&mut self, fd: types::Fd) -> Result<types::Filesize, types::Error> {
        let pos = self
            .transact()?
            .get_seekable(fd)
            .map(|File { position, .. }| position.load(Ordering::Relaxed))?;
        Ok(pos)
    }

    #[instrument(skip(self))]
    async fn fd_readdir<'a>(
        &mut self,
        fd: types::Fd,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, types::Error> {
        let fd = self.get_dir_fd(fd)?;
        let stream = self.read_directory(fd.borrowed()).await.map_err(|e| {
            e.try_into()
                .context("failed to call `read-directory`")
                .unwrap_or_else(types::Error::trap)
        })?;
        let dir_metadata_hash = self.metadata_hash(fd.borrowed()).await.map_err(|e| {
            e.try_into()
                .context("failed to call `metadata-hash`")
                .unwrap_or_else(types::Error::trap)
        })?;
        let cookie = cookie.try_into().map_err(|_| types::Errno::Overflow)?;

        let head = [
            (
                types::Dirent {
                    d_next: 1u64.to_le(),
                    d_ino: dir_metadata_hash.lower.to_le(),
                    d_type: types::Filetype::Directory,
                    d_namlen: 1u32.to_le(),
                },
                ".".into(),
            ),
            (
                types::Dirent {
                    d_next: 2u64.to_le(),
                    d_ino: dir_metadata_hash.lower.to_le(), // NOTE: incorrect, but legacy implementation returns `fd` inode here
                    d_type: types::Filetype::Directory,
                    d_namlen: 2u32.to_le(),
                },
                "..".into(),
            ),
        ];

        let mut dir = Vec::new();
        for (entry, d_next) in self
            .table()
            // remove iterator from table and use it directly:
            .delete(stream)?
            .into_iter()
            .zip(3u64..)
        {
            let filesystem::DirectoryEntry { type_, name } = entry.map_err(|e| {
                e.try_into()
                    .context("failed to inspect `read-directory` entry")
                    .unwrap_or_else(types::Error::trap)
            })?;
            let metadata_hash = self
                .metadata_hash_at(fd.borrowed(), filesystem::PathFlags::empty(), name.clone())
                .await
                .map_err(|e| {
                    e.try_into()
                        .context("failed to call `metadata-hash-at`")
                        .unwrap_or_else(types::Error::trap)
                })?;
            let d_type = type_.try_into().map_err(types::Error::trap)?;
            let d_namlen: u32 = name.len().try_into().map_err(|_| types::Errno::Overflow)?;
            dir.push((
                types::Dirent {
                    d_next: d_next.to_le(),
                    d_ino: metadata_hash.lower.to_le(),
                    d_type, // endian-invariant
                    d_namlen: d_namlen.to_le(),
                },
                name,
            ))
        }

        // assume that `types::Dirent` size always fits in `u32`
        const DIRENT_SIZE: u32 = size_of::<types::Dirent>() as _;
        assert_eq!(
            types::Dirent::guest_size(),
            DIRENT_SIZE,
            "Dirent guest repr and host repr should match"
        );
        let mut buf = *buf;
        let mut cap = buf_len;
        for (ref entry, path) in head.into_iter().chain(dir.into_iter()).skip(cookie) {
            let mut path = path.into_bytes();
            assert_eq!(
                1,
                size_of_val(&entry.d_type),
                "Dirent member d_type should be endian-invariant"
            );
            let entry_len = cap.min(DIRENT_SIZE);
            let entry = entry as *const _ as _;
            let entry = unsafe { slice::from_raw_parts(entry, entry_len as _) };
            cap = cap.checked_sub(entry_len).unwrap();
            buf = write_bytes(buf, entry)?;
            if cap == 0 {
                return Ok(buf_len);
            }

            if let Ok(cap) = cap.try_into() {
                // `path` cannot be longer than `usize`, only truncate if `cap` fits in `usize`
                path.truncate(cap);
            }
            cap = cap.checked_sub(path.len() as _).unwrap();
            buf = write_bytes(buf, &path)?;
            if cap == 0 {
                return Ok(buf_len);
            }
        }
        Ok(buf_len.checked_sub(cap).unwrap())
    }

    #[instrument(skip(self))]
    async fn path_create_directory<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let dirfd = self.get_dir_fd(dirfd)?;
        let path = read_string(path)?;
        self.create_directory_at(dirfd.borrowed(), path)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `create-directory-at`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    /// Return the attributes of a file or directory.
    /// NOTE: This is similar to `stat` in POSIX.
    #[instrument(skip(self))]
    async fn path_filestat_get<'a>(
        &mut self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
    ) -> Result<types::Filestat, types::Error> {
        let dirfd = self.get_dir_fd(dirfd)?;
        let path = read_string(path)?;
        let filesystem::DescriptorStat {
            type_,
            link_count: nlink,
            size,
            data_access_timestamp,
            data_modification_timestamp,
            status_change_timestamp,
        } = self
            .stat_at(dirfd.borrowed(), flags.into(), path.clone())
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `stat-at`")
                    .unwrap_or_else(types::Error::trap)
            })?;
        let metadata_hash = self
            .metadata_hash_at(dirfd, flags.into(), path)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `metadata-hash-at`")
                    .unwrap_or_else(types::Error::trap)
            })?;
        let filetype = type_.try_into().map_err(types::Error::trap)?;
        let zero = wall_clock::Datetime {
            seconds: 0,
            nanoseconds: 0,
        };
        let atim = data_access_timestamp.unwrap_or(zero).try_into()?;
        let mtim = data_modification_timestamp.unwrap_or(zero).try_into()?;
        let ctim = status_change_timestamp.unwrap_or(zero).try_into()?;
        Ok(types::Filestat {
            dev: 1,
            ino: metadata_hash.lower,
            filetype,
            nlink,
            size,
            atim,
            mtim,
            ctim,
        })
    }

    /// Adjust the timestamps of a file or directory.
    /// NOTE: This is similar to `utimensat` in POSIX.
    #[instrument(skip(self))]
    async fn path_filestat_set_times<'a>(
        &mut self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), types::Error> {
        let atim = systimespec(
            fst_flags.contains(types::Fstflags::ATIM),
            atim,
            fst_flags.contains(types::Fstflags::ATIM_NOW),
        )?;
        let mtim = systimespec(
            fst_flags.contains(types::Fstflags::MTIM),
            mtim,
            fst_flags.contains(types::Fstflags::MTIM_NOW),
        )?;

        let dirfd = self.get_dir_fd(dirfd)?;
        let path = read_string(path)?;
        self.set_times_at(dirfd, flags.into(), path, atim, mtim)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `set-times-at`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    /// Create a hard link.
    /// NOTE: This is similar to `linkat` in POSIX.
    #[instrument(skip(self))]
    async fn path_link<'a>(
        &mut self,
        src_fd: types::Fd,
        src_flags: types::Lookupflags,
        src_path: &GuestPtr<'a, str>,
        target_fd: types::Fd,
        target_path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let src_fd = self.get_dir_fd(src_fd)?;
        let target_fd = self.get_dir_fd(target_fd)?;
        let src_path = read_string(src_path)?;
        let target_path = read_string(target_path)?;
        self.link_at(src_fd, src_flags.into(), src_path, target_fd, target_path)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `link-at`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    /// Open a file or directory.
    /// NOTE: This is similar to `openat` in POSIX.
    #[instrument(skip(self))]
    async fn path_open<'a>(
        &mut self,
        dirfd: types::Fd,
        dirflags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
        oflags: types::Oflags,
        fs_rights_base: types::Rights,
        _fs_rights_inheriting: types::Rights,
        fdflags: types::Fdflags,
    ) -> Result<types::Fd, types::Error> {
        let path = read_string(path)?;

        let mut flags = filesystem::DescriptorFlags::empty();
        if fs_rights_base.contains(types::Rights::FD_READ) {
            flags |= filesystem::DescriptorFlags::READ;
        }
        if fs_rights_base.contains(types::Rights::FD_WRITE) {
            flags |= filesystem::DescriptorFlags::WRITE;
        }
        if fdflags.contains(types::Fdflags::SYNC) {
            flags |= filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC;
        }
        if fdflags.contains(types::Fdflags::DSYNC) {
            flags |= filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC;
        }
        if fdflags.contains(types::Fdflags::RSYNC) {
            flags |= filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC;
        }

        let t = self.transact()?;
        let dirfd = match t.get_descriptor(dirfd)? {
            Descriptor::Directory { fd, .. } => fd.borrowed(),
            Descriptor::File(_) => return Err(types::Errno::Notdir.into()),
            _ => return Err(types::Errno::Badf.into()),
        };
        drop(t);
        let fd = self
            .open_at(dirfd, dirflags.into(), path, oflags.into(), flags)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `open-at`")
                    .unwrap_or_else(types::Error::trap)
            })?;
        let mut t = self.transact()?;
        let desc = match t.view.table().get(&fd)? {
            crate::preview2::filesystem::Descriptor::Dir(_) => Descriptor::Directory {
                fd,
                preopen_path: None,
            },
            crate::preview2::filesystem::Descriptor::File(_) => Descriptor::File(File {
                fd,
                position: Default::default(),
                append: fdflags.contains(types::Fdflags::APPEND),
                blocking_mode: BlockingMode::from_fdflags(&fdflags),
            }),
        };
        let fd = t.descriptors.push(desc)?;
        Ok(fd.into())
    }

    /// Read the contents of a symbolic link.
    /// NOTE: This is similar to `readlinkat` in POSIX.
    #[instrument(skip(self))]
    async fn path_readlink<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, types::Error> {
        let dirfd = self.get_dir_fd(dirfd)?;
        let path = read_string(path)?;
        let mut path = self
            .readlink_at(dirfd, path)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `readlink-at`")
                    .unwrap_or_else(types::Error::trap)
            })?
            .into_bytes();
        if let Ok(buf_len) = buf_len.try_into() {
            // `path` cannot be longer than `usize`, only truncate if `buf_len` fits in `usize`
            path.truncate(buf_len);
        }
        let n = path.len().try_into().map_err(|_| types::Errno::Overflow)?;
        write_bytes(buf, &path)?;
        Ok(n)
    }

    #[instrument(skip(self))]
    async fn path_remove_directory<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let dirfd = self.get_dir_fd(dirfd)?;
        let path = read_string(path)?;
        self.remove_directory_at(dirfd, path).await.map_err(|e| {
            e.try_into()
                .context("failed to call `remove-directory-at`")
                .unwrap_or_else(types::Error::trap)
        })
    }

    /// Rename a file or directory.
    /// NOTE: This is similar to `renameat` in POSIX.
    #[instrument(skip(self))]
    async fn path_rename<'a>(
        &mut self,
        src_fd: types::Fd,
        src_path: &GuestPtr<'a, str>,
        dest_fd: types::Fd,
        dest_path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let src_fd = self.get_dir_fd(src_fd)?;
        let dest_fd = self.get_dir_fd(dest_fd)?;
        let src_path = read_string(src_path)?;
        let dest_path = read_string(dest_path)?;
        self.rename_at(src_fd, src_path, dest_fd, dest_path)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `rename-at`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    #[instrument(skip(self))]
    async fn path_symlink<'a>(
        &mut self,
        src_path: &GuestPtr<'a, str>,
        dirfd: types::Fd,
        dest_path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let dirfd = self.get_dir_fd(dirfd)?;
        let src_path = read_string(src_path)?;
        let dest_path = read_string(dest_path)?;
        self.symlink_at(dirfd.borrowed(), src_path, dest_path)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `symlink-at`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    #[instrument(skip(self))]
    async fn path_unlink_file<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        let dirfd = self.get_dir_fd(dirfd)?;
        let path = path.as_cow()?.to_string();
        self.unlink_file_at(dirfd.borrowed(), path)
            .await
            .map_err(|e| {
                e.try_into()
                    .context("failed to call `unlink-file-at`")
                    .unwrap_or_else(types::Error::trap)
            })
    }

    #[instrument(skip(self))]
    async fn poll_oneoff<'a>(
        &mut self,
        subs: &GuestPtr<'a, types::Subscription>,
        events: &GuestPtr<'a, types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size, types::Error> {
        if nsubscriptions == 0 {
            // Indefinite sleeping is not supported in preview1.
            return Err(types::Errno::Inval.into());
        }
        let subs = subs.as_array(nsubscriptions);
        let events = events.as_array(nsubscriptions);

        let n = usize::try_from(nsubscriptions).unwrap_or(usize::MAX);
        let mut pollables = Vec::with_capacity(n);
        for sub in subs.iter() {
            let sub = sub?.read()?;
            let p = match sub.u {
                types::SubscriptionU::Clock(types::SubscriptionClock {
                    id,
                    timeout,
                    flags,
                    ..
                }) => {
                    let absolute = flags.contains(types::Subclockflags::SUBSCRIPTION_CLOCK_ABSTIME);
                    let (timeout, absolute) = match id {
                        types::Clockid::Monotonic => (timeout, absolute),
                        types::Clockid::Realtime if !absolute => (timeout, false),
                        types::Clockid::Realtime => {
                            let now = wall_clock::Host::now(self)
                                .context("failed to call `wall_clock::now`")
                                .map_err(types::Error::trap)?;

                            // Convert `timeout` to `Datetime` format.
                            let seconds = timeout / 1_000_000_000;
                            let nanoseconds = timeout % 1_000_000_000;

                            let timeout = if now.seconds < seconds
                                || now.seconds == seconds
                                    && u64::from(now.nanoseconds) < nanoseconds
                            {
                                // `now` is less than `timeout`, which is expressable as u64,
                                // substract the nanosecond counts directly
                                now.seconds * 1_000_000_000 + u64::from(now.nanoseconds) - timeout
                            } else {
                                0
                            };
                            (timeout, false)
                        }
                        _ => return Err(types::Errno::Inval.into()),
                    };
                    if absolute {
                        monotonic_clock::Host::subscribe_instant(self, timeout)
                            .context("failed to call `monotonic_clock::subscribe_instant`")
                            .map_err(types::Error::trap)?
                    } else {
                        monotonic_clock::Host::subscribe_duration(self, timeout)
                            .context("failed to call `monotonic_clock::subscribe_duration`")
                            .map_err(types::Error::trap)?
                    }
                }
                types::SubscriptionU::FdRead(types::SubscriptionFdReadwrite {
                    file_descriptor,
                }) => {
                    let stream = {
                        let t = self.transact()?;
                        let desc = t.get_descriptor(file_descriptor)?;
                        match desc {
                            Descriptor::Stdin { stream, .. } => stream.borrowed(),
                            Descriptor::File(File { fd, position, .. }) => {
                                let pos = position.load(Ordering::Relaxed);
                                let fd = fd.borrowed();
                                drop(t);
                                self.read_via_stream(fd, pos).map_err(|e| {
                                    e.try_into()
                                        .context("failed to call `read-via-stream`")
                                        .unwrap_or_else(types::Error::trap)
                                })?
                            }
                            // TODO: Support sockets
                            _ => return Err(types::Errno::Badf.into()),
                        }
                    };
                    streams::HostInputStream::subscribe(self, stream)
                        .context("failed to call `subscribe` on `input-stream`")
                        .map_err(types::Error::trap)?
                }
                types::SubscriptionU::FdWrite(types::SubscriptionFdReadwrite {
                    file_descriptor,
                }) => {
                    let stream = {
                        let t = self.transact()?;
                        let desc = t.get_descriptor(file_descriptor)?;
                        match desc {
                            Descriptor::Stdout { stream, .. }
                            | Descriptor::Stderr { stream, .. } => stream.borrowed(),
                            Descriptor::File(File {
                                fd,
                                position,
                                append,
                                ..
                            }) => {
                                let fd = fd.borrowed();
                                let position = position.clone();
                                let append = *append;
                                drop(t);
                                if append {
                                    self.append_via_stream(fd).map_err(|e| {
                                        e.try_into()
                                            .context("failed to call `append-via-stream`")
                                            .unwrap_or_else(types::Error::trap)
                                    })?
                                } else {
                                    let pos = position.load(Ordering::Relaxed);
                                    self.write_via_stream(fd, pos).map_err(|e| {
                                        e.try_into()
                                            .context("failed to call `write-via-stream`")
                                            .unwrap_or_else(types::Error::trap)
                                    })?
                                }
                            }
                            // TODO: Support sockets
                            _ => return Err(types::Errno::Badf.into()),
                        }
                    };
                    streams::HostOutputStream::subscribe(self, stream)
                        .context("failed to call `subscribe` on `output-stream`")
                        .map_err(types::Error::trap)?
                }
            };
            pollables.push(p);
        }
        let ready: HashSet<_> = self
            .poll(pollables)
            .await
            .context("failed to call `poll-oneoff`")
            .map_err(types::Error::trap)?
            .into_iter()
            .collect();

        let mut count: types::Size = 0;
        for (sub, event) in (0..)
            .zip(subs.iter())
            .filter_map(|(idx, sub)| ready.contains(&idx).then_some(sub))
            .zip(events.iter())
        {
            let sub = sub?.read()?;
            let event = event?;
            let e = match sub.u {
                types::SubscriptionU::Clock(..) => types::Event {
                    userdata: sub.userdata,
                    error: types::Errno::Success,
                    type_: types::Eventtype::Clock,
                    fd_readwrite: types::EventFdReadwrite {
                        flags: types::Eventrwflags::empty(),
                        nbytes: 0,
                    },
                },
                types::SubscriptionU::FdRead(types::SubscriptionFdReadwrite {
                    file_descriptor,
                }) => {
                    let t = self.transact()?;
                    let desc = t.get_descriptor(file_descriptor)?;
                    match desc {
                        Descriptor::Stdin { .. } => types::Event {
                            userdata: sub.userdata,
                            error: types::Errno::Success,
                            type_: types::Eventtype::FdRead,
                            fd_readwrite: types::EventFdReadwrite {
                                flags: types::Eventrwflags::empty(),
                                nbytes: 1,
                            },
                        },
                        Descriptor::File(File { fd, position, .. }) => {
                            let fd = fd.borrowed();
                            let position = position.clone();
                            drop(t);
                            match self.stat(fd).await? {
                                filesystem::DescriptorStat { size, .. } => {
                                    let pos = position.load(Ordering::Relaxed);
                                    let nbytes = size.saturating_sub(pos);
                                    types::Event {
                                        userdata: sub.userdata,
                                        error: types::Errno::Success,
                                        type_: types::Eventtype::FdRead,
                                        fd_readwrite: types::EventFdReadwrite {
                                            flags: if nbytes == 0 {
                                                types::Eventrwflags::FD_READWRITE_HANGUP
                                            } else {
                                                types::Eventrwflags::empty()
                                            },
                                            nbytes: 1,
                                        },
                                    }
                                }
                            }
                        }
                        // TODO: Support sockets
                        _ => return Err(types::Errno::Badf.into()),
                    }
                }
                types::SubscriptionU::FdWrite(types::SubscriptionFdReadwrite {
                    file_descriptor,
                }) => {
                    let t = self.transact()?;
                    let desc = t.get_descriptor(file_descriptor)?;
                    match desc {
                        Descriptor::Stdout { .. } | Descriptor::Stderr { .. } => types::Event {
                            userdata: sub.userdata,
                            error: types::Errno::Success,
                            type_: types::Eventtype::FdWrite,
                            fd_readwrite: types::EventFdReadwrite {
                                flags: types::Eventrwflags::empty(),
                                nbytes: 1,
                            },
                        },
                        Descriptor::File(_) => types::Event {
                            userdata: sub.userdata,
                            error: types::Errno::Success,
                            type_: types::Eventtype::FdWrite,
                            fd_readwrite: types::EventFdReadwrite {
                                flags: types::Eventrwflags::empty(),
                                nbytes: 1,
                            },
                        },
                        // TODO: Support sockets
                        _ => return Err(types::Errno::Badf.into()),
                    }
                }
            };
            event.write(e)?;
            count = count
                .checked_add(1)
                .ok_or_else(|| types::Error::from(types::Errno::Overflow))?
        }
        Ok(count)
    }

    #[instrument(skip(self))]
    fn proc_exit(&mut self, status: types::Exitcode) -> anyhow::Error {
        // Check that the status is within WASI's range.
        if status >= 126 {
            return anyhow::Error::msg("exit with invalid exit status outside of [0..126)");
        }
        crate::preview2::I32Exit(status as i32).into()
    }

    #[instrument(skip(self))]
    fn proc_raise(&mut self, _sig: types::Signal) -> Result<(), types::Error> {
        Err(types::Errno::Notsup.into())
    }

    #[instrument(skip(self))]
    fn sched_yield(&mut self) -> Result<(), types::Error> {
        // No such thing in preview 2. Intentionally left empty.
        Ok(())
    }

    #[instrument(skip(self))]
    fn random_get<'a>(
        &mut self,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
    ) -> Result<(), types::Error> {
        let rand = self
            .get_random_bytes(buf_len.into())
            .context("failed to call `get-random-bytes`")
            .map_err(types::Error::trap)?;
        write_bytes(buf, &rand)?;
        Ok(())
    }

    #[allow(unused_variables)]
    #[instrument(skip(self))]
    fn sock_accept(
        &mut self,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<types::Fd, types::Error> {
        todo!("preview1 sock_accept is not implemented")
    }

    #[allow(unused_variables)]
    #[instrument(skip(self))]
    fn sock_recv<'a>(
        &mut self,
        fd: types::Fd,
        ri_data: &types::IovecArray<'a>,
        ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), types::Error> {
        todo!("preview1 sock_recv is not implemented")
    }

    #[allow(unused_variables)]
    #[instrument(skip(self))]
    fn sock_send<'a>(
        &mut self,
        fd: types::Fd,
        si_data: &types::CiovecArray<'a>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size, types::Error> {
        todo!("preview1 sock_send is not implemented")
    }

    #[allow(unused_variables)]
    #[instrument(skip(self))]
    fn sock_shutdown(&mut self, fd: types::Fd, how: types::Sdflags) -> Result<(), types::Error> {
        todo!("preview1 sock_shutdown is not implemented")
    }
}

trait ResourceExt<T> {
    fn borrowed(&self) -> Resource<T>;
}

impl<T: 'static> ResourceExt<T> for Resource<T> {
    fn borrowed(&self) -> Resource<T> {
        Resource::new_borrow(self.rep())
    }
}
