use std::sync::{Arc, RwLock};
use wasi_common::pipe::{ReadPipe, WritePipe};

/// Input/Output stream redirects
#[derive(Clone)]
pub struct IoStreamRedirects {
    /// Standard input redirect.
    pub stdin: ReadPipe<std::io::Cursor<Vec<u8>>>,
    /// Standard output redirect.
    pub stdout: OutRedirect,
    /// Standard error redirect.
    pub stderr: OutRedirect,
}

/// Output redirect and lock.
#[derive(Clone)]
pub struct OutRedirect {
    /// Output redirect.
    pub out: WritePipe<Vec<u8>>,
    /// Lock for writing.
    pub lock: Arc<RwLock<Vec<u8>>>,
}

/// Prepares WASI pipes which redirect a component's output to
/// memory buffers.
pub fn prepare_io_redirects() -> anyhow::Result<IoStreamRedirects> {
    let stdin = ReadPipe::from(vec![]);

    let stdout_buf: Vec<u8> = vec![];
    let lock = Arc::new(RwLock::new(stdout_buf));
    let stdout = WritePipe::from_shared(lock.clone());
    let stdout = OutRedirect { out: stdout, lock };

    let stderr_buf: Vec<u8> = vec![];
    let lock = Arc::new(RwLock::new(stderr_buf));
    let stderr = WritePipe::from_shared(lock.clone());
    let stderr = OutRedirect { out: stderr, lock };

    Ok(IoStreamRedirects {
        stdin,
        stdout,
        stderr,
    })
}
