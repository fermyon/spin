use std::sync::{Arc, RwLock};

use wasi_common::pipe::WritePipe;

/// An in-memory stdio output buffer.
#[derive(Default)]
pub struct OutputBuffer(Arc<RwLock<Vec<u8>>>);

impl OutputBuffer {
    /// Takes the buffered output from this buffer.
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut *self.0.write().unwrap())
    }

    pub(crate) fn writer(&self) -> WritePipe<Vec<u8>> {
        WritePipe::from_shared(self.0.clone())
    }
}
