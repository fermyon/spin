use std::io::{Read, Write};

use spin_manifest::CoreComponent;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{
    pipe::{ReadPipe, WritePipe},
    WasiFile,
};

use crate::Component;

/// Configuration for the WASI stdio (stdin, stdout, stderr) of a component.
#[derive(Clone)]
pub struct ComponentStdio {
    stdin: StdInput,
    stdout: StdOutput,
    stderr: StdOutput,
}

impl Default for ComponentStdio {
    fn default() -> Self {
        Self {
            stdin: StdInput::Null,
            stdout: StdOutput::Null,
            stderr: StdOutput::Log,
        }
    }
}

/// Configuration overrides for WASI stdio of a component. Used by trigger executors
/// that need control of stdio stream(s).
#[derive(Default)]
pub struct ComponentStdioOverrides {
    stdin: Option<Box<dyn WasiFile>>,
    stdout: Option<Box<dyn WasiFile>>,
    stderr: Option<Box<dyn WasiFile>>,
}

impl ComponentStdioOverrides {
    /// Set the stdin override to the given WasiFile.
    pub fn stdin(mut self, f: impl WasiFile + 'static) -> Self {
        self.stdin = Some(Box::new(f));
        self
    }

    /// Set the stdout override to the given WasiFile.
    pub fn stdout(mut self, f: impl WasiFile + 'static) -> Self {
        self.stdout = Some(Box::new(f));
        self
    }

    /// Set the stderr override to the given WasiFile.
    pub fn stderr(mut self, f: impl WasiFile + 'static) -> Self {
        self.stderr = Some(Box::new(f));
        self
    }
}

/// Configuration for the behavior of a component's WASI input stream (stdin).
#[derive(Clone)]
pub enum StdInput {
    /// Input is empty.
    Null,
    /// Input is read from the given `ReadPipe`.
    Pipe(ReadPipe<Box<dyn Read + Send + Sync>>),
}

impl StdInput {
    /// Wrap the given reader for use as a StdInput.
    pub fn pipe(reader: impl Read + Send + Sync + 'static) -> Self {
        Self::Pipe(ReadPipe::new(Box::new(reader)))
    }

    fn to_wasi_file(&self) -> Option<Box<dyn WasiFile>> {
        match self {
            StdInput::Null => None,
            StdInput::Pipe(pipe) => Some(Box::new(pipe.clone())),
        }
    }
}

/// Configuration for the behavior of a component's WASI output streams (stdout/stderr).
#[derive(Clone)]
pub enum StdOutput {
    /// Output is discarded.
    Null,
    /// Output is logged via `tracing`.
    Log,
    /// Output is written to the given `WritePipe`.
    Pipe(WritePipe<Box<dyn Write + Send + Sync>>),
}

impl StdOutput {
    /// Wrap the given writer for use as a StdOutput.
    pub fn pipe(writer: impl Write + Send + Sync + 'static) -> Self {
        Self::Pipe(WritePipe::new(Box::new(writer)))
    }

    fn to_wasi_file(
        &self,
        core: &CoreComponent,
        stream: &'static str,
    ) -> Option<Box<dyn WasiFile>> {
        match self {
            Self::Null => None,
            Self::Log => Some(Box::new(log::LoggingWasiFile::new(core, stream))),
            Self::Pipe(pipe) => Some(Box::new(pipe.clone())),
        }
    }
}

pub(crate) fn apply_component_stdio<T: Default>(
    mut builder: WasiCtxBuilder,
    component: &Component<T>,
    overrides: ComponentStdioOverrides,
) -> WasiCtxBuilder {
    // stdin
    if let Some(wasi_file) = overrides
        .stdin
        .or_else(|| component.stdio.stdin.to_wasi_file())
    {
        builder = builder.stdin(wasi_file);
    }
    // stdout
    if let Some(wasi_file) = overrides.stdout.or_else(|| {
        component
            .stdio
            .stdout
            .to_wasi_file(&component.core, "stdout")
    }) {
        builder = builder.stdout(wasi_file);
    }
    // stderr
    if let Some(wasi_file) = overrides.stderr.or_else(|| {
        component
            .stdio
            .stderr
            .to_wasi_file(&component.core, "stderr")
    }) {
        builder = builder.stderr(wasi_file);
    }
    builder
}

mod log {
    use std::{
        any::Any,
        io::{self, Write},
        sync::Mutex,
    };

    use anyhow::Context;
    use async_trait::async_trait;
    use spin_manifest::CoreComponent;
    use tracing::{event, event_enabled, warn, Level};
    use wasi_common::{
        file::{Advice, FdFlags, FileType, Filestat, WasiFile},
        Error, ErrorExt, SystemTimeSpec,
    };

    pub(crate) struct LoggingWasiFile {
        component_id: String,
        stream: &'static str,
        unprocessed: Mutex<Vec<u8>>,
    }

    impl LoggingWasiFile {
        const TARGET: &'static str = "component-log";
        const LEVEL: Level = Level::INFO;

        pub fn new(core: &CoreComponent, stream: &'static str) -> Self {
            Self {
                component_id: core.id.to_string(),
                stream,
                unprocessed: Default::default(),
            }
        }

        fn write_line(&self, line: &[u8]) {
            match std::str::from_utf8(line) {
                Ok(message) => {
                    event!(
                        target: LoggingWasiFile::TARGET,
                        LoggingWasiFile::LEVEL,
                        message = message.trim_end(),
                        component_id = self.component_id.as_str(),
                        stream = self.stream,
                    );
                }
                Err(err) => {
                    warn!(
                        target: LoggingWasiFile::TARGET,
                        message = format!("invalid log message encoding: {}", err).as_str(),
                        component_id = self.component_id.as_str(),
                        stream = self.stream,
                    );
                }
            }
        }
    }

    impl Drop for LoggingWasiFile {
        fn drop(&mut self) {
            if let Ok(buf) = self.unprocessed.try_lock() {
                if !buf.is_empty() {
                    self.write_line(&buf[..]);
                }
            } else {
            }
        }
    }

    // Adapted from WritePipe
    #[allow(unused_variables)]
    #[async_trait]
    impl WasiFile for LoggingWasiFile {
        async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
            if event_enabled!(
                target: LoggingWasiFile::TARGET,
                LoggingWasiFile::LEVEL,
                component_id,
                stream,
            ) {
                let mut unprocessed = self.unprocessed.lock().unwrap();
                let mut n: u64 = 0;
                for buf in bufs {
                    let mut buf = buf.as_ref();
                    n += buf.len() as u64;
                    if unprocessed.is_empty() {
                        // Fast path: nothing left over from previous writes; don't need to copy
                        while let Some(nl_idx) = buf.iter().position(|b| *b == b'\n') {
                            let line;
                            (line, buf) = buf.split_at(nl_idx + 1);
                            self.write_line(line);
                        }
                    }
                    unprocessed.extend_from_slice(buf);
                }
                // Slow path: log line(s) split over multiple writes
                while let Some(nl_idx) = unprocessed.iter().position(|b| *b == b'\n') {
                    let rest = unprocessed.split_off(nl_idx + 1);
                    let line = std::mem::replace(&mut *unprocessed, rest);
                    self.write_line(&line[..]);
                }
                Ok(n)
            } else {
                std::io::sink()
                    .write_vectored(bufs)
                    .map(|n| n as u64)
                    .context("infallible")
            }
        }

        // === Unchanged from WritePipe below this line ===

        fn as_any(&self) -> &dyn Any {
            self
        }
        async fn datasync(&self) -> Result<(), Error> {
            Ok(())
        }
        async fn sync(&self) -> Result<(), Error> {
            Ok(())
        }
        async fn get_filetype(&self) -> Result<FileType, Error> {
            Ok(FileType::Pipe)
        }
        async fn get_fdflags(&self) -> Result<FdFlags, Error> {
            Ok(FdFlags::APPEND)
        }
        async fn set_fdflags(&mut self, _fdflags: FdFlags) -> Result<(), Error> {
            Err(Error::badf())
        }
        async fn get_filestat(&self) -> Result<Filestat, Error> {
            Ok(Filestat {
                device_id: 0,
                inode: 0,
                filetype: self.get_filetype().await?,
                nlink: 0,
                size: 0, // XXX no way to get a size out of a Write :(
                atim: None,
                mtim: None,
                ctim: None,
            })
        }
        async fn set_filestat_size(&self, _size: u64) -> Result<(), Error> {
            Err(Error::badf())
        }
        async fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
            Err(Error::badf())
        }
        async fn allocate(&self, offset: u64, len: u64) -> Result<(), Error> {
            Err(Error::badf())
        }
        async fn read_vectored<'a>(&self, bufs: &mut [io::IoSliceMut<'a>]) -> Result<u64, Error> {
            Err(Error::badf())
        }
        async fn read_vectored_at<'a>(
            &self,
            bufs: &mut [io::IoSliceMut<'a>],
            offset: u64,
        ) -> Result<u64, Error> {
            Err(Error::badf())
        }
        async fn write_vectored_at<'a>(
            &self,
            bufs: &[io::IoSlice<'a>],
            offset: u64,
        ) -> Result<u64, Error> {
            Err(Error::badf())
        }
        async fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
            Err(Error::badf())
        }
        async fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
            Err(Error::badf())
        }
        async fn set_times(
            &self,
            atime: Option<SystemTimeSpec>,
            mtime: Option<SystemTimeSpec>,
        ) -> Result<(), Error> {
            Err(Error::badf())
        }
        async fn num_ready_bytes(&self) -> Result<u64, Error> {
            Ok(0)
        }
        fn isatty(&self) -> bool {
            false
        }
        async fn readable(&self) -> Result<(), Error> {
            Err(Error::badf())
        }
        async fn writable(&self) -> Result<(), Error> {
            Err(Error::badf())
        }

        async fn sock_accept(&mut self, fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
            Err(Error::badf())
        }
    }
}
