use wasmtime_wasi::preview2::{pipe::MemoryOutputPipe, HostOutputStream};

/// An in-memory stdio output buffer.
pub struct OutputBuffer(MemoryOutputPipe);

impl OutputBuffer {
    /// Clones the buffered output from this buffer.
    pub fn contents(&self) -> bytes::Bytes {
        self.0.contents()
    }

    pub(crate) fn writer(&self) -> impl HostOutputStream {
        self.0.clone()
    }
}

impl Default for OutputBuffer {
    fn default() -> Self {
        Self(MemoryOutputPipe::new(usize::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn take_what_you_write() {
        let buf = OutputBuffer::default();
        buf.writer().write(b"foo".to_vec().into()).unwrap();
        assert_eq!(buf.contents().as_ref(), b"foo");
    }
}
