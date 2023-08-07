use wasmtime_wasi::preview2::{pipe::MemoryOutputPipe, HostOutputStream};

/// An in-memory stdio output buffer.
#[derive(Clone)]
pub struct OutputBuffer(MemoryOutputPipe);

impl OutputBuffer {
    /// Takes the buffered output from this buffer.
    pub fn take(&mut self) -> Vec<u8> {
        self.0.contents().to_vec()
    }

    pub(crate) fn writer(&self) -> impl HostOutputStream {
        self.0.clone()
    }
}

impl Default for OutputBuffer {
    fn default() -> Self {
        Self(MemoryOutputPipe::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn take_what_you_write() {
        let mut buf = OutputBuffer::default();
        buf.writer().write(b"foo".to_vec().into()).unwrap();
        assert_eq!(buf.take(), b"foo");
    }
}
