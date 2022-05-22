use cap_std::fs::File as CapFile;
use std::{
    collections::HashSet,
    fmt::Debug,
    fs::{File, OpenOptions},
    io::{LineWriter, Write},
    path::PathBuf,
    sync::{Arc, RwLock, RwLockReadGuard},
};
use wasi_common::{
    pipe::{ReadPipe, WritePipe},
    WasiFile,
};
use wasmtime_wasi::sync::file::File as WasmtimeFile;

/// Prepares a WASI pipe which writes to a memory buffer, optionally
/// copying to the specified output stream.
pub fn redirect_to_mem_buffer(
    follow: Follow,
) -> (WritePipe<WriteDestinations>, Arc<RwLock<WriteDestinations>>) {
    let immediate = follow.writer();

    let buffer: Vec<u8> = vec![];
    let std_dests = WriteDestinations { buffer, immediate };
    let lock = Arc::new(RwLock::new(std_dests));
    let std_pipe = WritePipe::from_shared(lock.clone());

    (std_pipe, lock)
}

/// Which components should have their logs followed on stdout/stderr.
#[derive(Clone, Debug)]
pub enum FollowComponents {
    /// No components should have their logs followed.
    None,
    /// Only the specified components should have their logs followed.
    Named(HashSet<String>),
    /// All components should have their logs followed.
    All,
}

impl FollowComponents {
    /// Whether a given component should have its logs followed on stdout/stderr.
    pub fn should_follow(&self, component_id: &str) -> bool {
        match self {
            Self::None => false,
            Self::All => true,
            Self::Named(ids) => ids.contains(component_id),
        }
    }
}

/// The buffers in which Wasm module output has been saved.
pub trait OutputBuffers {
    /// The buffer in which stdout has been saved.
    fn stdout(&self) -> &[u8];
    /// The buffer in which stderr has been saved.
    fn stderr(&self) -> &[u8];
}

/// Wrapper around File with a convenient PathBuf for cloning
pub struct PipeFile(pub File, pub PathBuf);

impl PipeFile {
    /// Constructs an instance from a file, and the PathBuf to that file.
    pub fn new(file: File, path: PathBuf) -> Self {
        Self(file, path)
    }
}

impl std::fmt::Debug for PipeFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipeFile")
            .field("File", &self.0)
            .field("PathBuf", &self.1)
            .finish()
    }
}

impl Clone for PipeFile {
    fn clone(&self) -> Self {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.1)
            .unwrap();
        Self(f, self.1.clone())
    }
}

/// CustomIoPipes that can be passed to `ExecutionContextConfiguration`
/// to direct out and err
#[derive(Clone, Debug, Default)]
pub struct CustomLogPipes {
    /// in pipe (file and pathbuf)
    pub stdin_pipe: Option<PipeFile>,
    /// out pipe (file and pathbuf)
    pub stdout_pipe: Option<PipeFile>,
    /// err pipe (file and pathbuf)
    pub stderr_pipe: Option<PipeFile>,
}

impl CustomLogPipes {
    /// Constructs an instance from a set of PipeFile objects.
    pub fn new(
        stdin_pipe: Option<PipeFile>,
        stdout_pipe: Option<PipeFile>,
        stderr_pipe: Option<PipeFile>,
    ) -> Self {
        Self {
            stdin_pipe,
            stdout_pipe,
            stderr_pipe,
        }
    }
}

/// Types of ModuleIoRedirects
#[derive(Clone, Debug)]
pub enum ModuleIoRedirectsTypes {
    /// This will signal the executor to use `capture_io_to_memory_default()`
    Default,
    /// This will signal the executor to use `capture_io_to_memory_files()`
    FromFiles(CustomLogPipes),
}

impl Default for ModuleIoRedirectsTypes {
    fn default() -> Self {
        Self::Default
    }
}

/// A set of redirected standard I/O streams with which
/// a Wasm module is to be run.
pub struct ModuleIoRedirects {
    /// pipes for ModuleIoRedirects
    pub pipes: RedirectPipes,
    /// read handles for ModuleIoRedirects
    pub read_handles: RedirectReadHandles,
}

impl Default for ModuleIoRedirects {
    fn default() -> Self {
        Self::new(false)
    }
}

impl ModuleIoRedirects {
    /// Constructs the ModuleIoRedirects, and RedirectReadHandles instances the default way
    pub fn new(follow: bool) -> Self {
        let rrh = RedirectReadHandles::new(follow);

        let in_stdpipe: Box<dyn WasiFile> = Box::new(ReadPipe::from(vec![]));
        let out_stdpipe: Box<dyn WasiFile> = Box::new(WritePipe::from_shared(rrh.stdout.clone()));
        let err_stdpipe: Box<dyn WasiFile> = Box::new(WritePipe::from_shared(rrh.stderr.clone()));

        Self {
            pipes: RedirectPipes {
                stdin: in_stdpipe,
                stdout: out_stdpipe,
                stderr: err_stdpipe,
            },
            read_handles: rrh,
        }
    }

    /// Constructs the ModuleIoRedirects, and RedirectReadHandles instances from `File`s directly
    pub fn new_from_files(
        stdin_file: Option<File>,
        stdout_file: Option<File>,
        stderr_file: Option<File>,
    ) -> Self {
        let rrh = RedirectReadHandles::new(true);

        let in_stdpipe: Box<dyn WasiFile> = match stdin_file {
            Some(inf) => Box::new(WasmtimeFile::from_cap_std(CapFile::from_std(inf))),
            None => Box::new(ReadPipe::from(vec![])),
        };
        let out_stdpipe: Box<dyn WasiFile> = match stdout_file {
            Some(ouf) => Box::new(WasmtimeFile::from_cap_std(CapFile::from_std(ouf))),
            None => Box::new(WritePipe::from_shared(rrh.stdout.clone())),
        };
        let err_stdpipe: Box<dyn WasiFile> = match stderr_file {
            Some(erf) => Box::new(WasmtimeFile::from_cap_std(CapFile::from_std(erf))),
            None => Box::new(WritePipe::from_shared(rrh.stderr.clone())),
        };

        Self {
            pipes: RedirectPipes {
                stdin: in_stdpipe,
                stdout: out_stdpipe,
                stderr: err_stdpipe,
            },
            read_handles: rrh,
        }
    }
}

/// Pipes from `ModuleIoRedirects`
pub struct RedirectPipes {
    pub(crate) stdin: Box<dyn WasiFile>,
    pub(crate) stdout: Box<dyn WasiFile>,
    pub(crate) stderr: Box<dyn WasiFile>,
}

impl RedirectPipes {
    /// Constructs an instance from a set of WasiFile objects.
    pub fn new(
        stdin: Box<dyn WasiFile>,
        stdout: Box<dyn WasiFile>,
        stderr: Box<dyn WasiFile>,
    ) -> Self {
        Self {
            stdin,
            stdout,
            stderr,
        }
    }
}

/// The destinations to which redirected module output will be written.
/// Used for subsequently reading back the output.
pub struct RedirectReadHandles {
    stdout: Arc<RwLock<WriteDestinations>>,
    stderr: Arc<RwLock<WriteDestinations>>,
}

impl Default for RedirectReadHandles {
    fn default() -> Self {
        Self::new(false)
    }
}

impl RedirectReadHandles {
    /// Creates a new RedirectReadHandles instance
    pub fn new(follow: bool) -> Self {
        let out_immediate = Follow::stdout(follow).writer();
        let err_immediate = Follow::stderr(follow).writer();

        let out_buffer: Vec<u8> = vec![];
        let err_buffer: Vec<u8> = vec![];

        let out_std_dests = WriteDestinations {
            buffer: out_buffer,
            immediate: out_immediate,
        };
        let err_std_dests = WriteDestinations {
            buffer: err_buffer,
            immediate: err_immediate,
        };

        Self {
            stdout: Arc::new(RwLock::new(out_std_dests)),
            stderr: Arc::new(RwLock::new(err_std_dests)),
        }
    }
    /// Acquires a read lock for the in-memory output buffers.
    pub fn read(&self) -> impl OutputBuffers + '_ {
        RedirectReadHandlesLock {
            stdout: self.stdout.read().unwrap(),
            stderr: self.stderr.read().unwrap(),
        }
    }
}

struct RedirectReadHandlesLock<'a> {
    stdout: RwLockReadGuard<'a, WriteDestinations>,
    stderr: RwLockReadGuard<'a, WriteDestinations>,
}

impl<'a> OutputBuffers for RedirectReadHandlesLock<'a> {
    fn stdout(&self) -> &[u8] {
        self.stdout.buffer()
    }
    fn stderr(&self) -> &[u8] {
        self.stderr.buffer()
    }
}

/// Indicates whether a memory redirect should also pipe the output to
/// the console so it can be followed live.
pub enum Follow {
    /// Do not pipe to console - only write to memory.
    None,
    /// Also pipe to stdout.
    Stdout,
    /// Also pipe to stderr.
    Stderr,
}

impl Follow {
    pub(crate) fn writer(&self) -> Box<dyn Write + Send + Sync> {
        match self {
            Self::None => Box::new(DiscardingWriter),
            Self::Stdout => Box::new(LineWriter::new(std::io::stdout())),
            Self::Stderr => Box::new(LineWriter::new(std::io::stderr())),
        }
    }

    /// Follow on stdout if so specified.
    pub fn stdout(follow_on_stdout: bool) -> Self {
        if follow_on_stdout {
            Self::Stdout
        } else {
            Self::None
        }
    }

    /// Follow on stderr if so specified.
    pub fn stderr(follow_on_stderr: bool) -> Self {
        if follow_on_stderr {
            Self::Stderr
        } else {
            Self::None
        }
    }
}

/// The destinations to which a component writes an output stream.
pub struct WriteDestinations {
    buffer: Vec<u8>,
    immediate: Box<dyn Write + Send + Sync>,
}

impl WriteDestinations {
    /// The memory buffer to which a component writes an output stream.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl Write for WriteDestinations {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.buffer.write(buf)?;
        self.immediate.write_all(&buf[0..written])?;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.flush()?;
        self.immediate.flush()?;
        Ok(())
    }
}

struct DiscardingWriter;

impl Write for DiscardingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
