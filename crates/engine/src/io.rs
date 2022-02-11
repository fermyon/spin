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
