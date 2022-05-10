use std::{
    collections::HashSet,
    io::{LineWriter, Write},
    sync::{Arc, RwLock, RwLockReadGuard},
};

use wasi_common::{
    pipe::{ReadPipe, WritePipe},
    WasiFile,
};

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

/// A set of redirected standard I/O streams with which
/// a Wasm module is to be run.
pub struct ModuleIoRedirects {
    pub(crate) stdin: Box<dyn WasiFile>,
    pub(crate) stdout: Box<dyn WasiFile>,
    pub(crate) stderr: Box<dyn WasiFile>,
}

impl ModuleIoRedirects {
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

impl RedirectReadHandles {
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

/// Prepares WASI pipes which redirect a component's output to
/// memory buffers.
pub fn capture_io_to_memory(
    follow_on_stdout: bool,
    follow_on_stderr: bool,
) -> (ModuleIoRedirects, RedirectReadHandles) {
    let stdout_follow = Follow::stdout(follow_on_stdout);
    let stderr_follow = Follow::stderr(follow_on_stderr);

    let stdin = ReadPipe::from(vec![]);

    let (stdout_pipe, stdout_lock) = redirect_to_mem_buffer(stdout_follow);

    let (stderr_pipe, stderr_lock) = redirect_to_mem_buffer(stderr_follow);

    let redirects = ModuleIoRedirects {
        stdin: Box::new(stdin),
        stdout: Box::new(stdout_pipe),
        stderr: Box::new(stderr_pipe),
    };

    let outputs = RedirectReadHandles {
        stdout: stdout_lock,
        stderr: stderr_lock,
    };

    (redirects, outputs)
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
