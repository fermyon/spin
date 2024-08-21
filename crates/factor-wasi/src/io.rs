use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use spin_factors::anyhow;
use wasmtime_wasi::{
    HostInputStream, HostOutputStream, StdinStream, StdoutStream, StreamError, Subscribe,
};

/// A [`HostOutputStream`] that writes to a `Write` type.
///
/// `StdinStream::stream` and `StdoutStream::new` can be called more than once in components
/// which are composed of multiple subcomponents, since each subcomponent will potentially want
/// its own handle. This means the streams need to be shareable. The easiest way to do that is
/// provide cloneable implementations of streams which operate synchronously.
///
/// Note that this amounts to doing synchronous I/O in an asynchronous context, which we'd normally
/// prefer to avoid, but the properly asynchronous implementations Host{In|Out}putStream based on
/// `AsyncRead`/`AsyncWrite`` are quite hairy and probably not worth it for "normal" stdio streams in
/// Spin. If this does prove to be a performance bottleneck, though, we can certainly revisit it.
pub struct PipedWriteStream<T>(Arc<Mutex<T>>);

impl<T> PipedWriteStream<T> {
    pub fn new(inner: T) -> Self {
        Self(Arc::new(Mutex::new(inner)))
    }
}

impl<T> Clone for PipedWriteStream<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Write + Send + Sync + 'static> HostOutputStream for PipedWriteStream<T> {
    fn write(&mut self, bytes: bytes::Bytes) -> Result<(), StreamError> {
        self.0
            .lock()
            .unwrap()
            .write_all(&bytes)
            .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn flush(&mut self) -> Result<(), StreamError> {
        self.0
            .lock()
            .unwrap()
            .flush()
            .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn check_write(&mut self) -> Result<usize, StreamError> {
        Ok(1024 * 1024)
    }
}

impl<T: Write + Send + Sync + 'static> StdoutStream for PipedWriteStream<T> {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

#[async_trait]
impl<T: Write + Send + Sync + 'static> Subscribe for PipedWriteStream<T> {
    async fn ready(&mut self) {}
}

/// A [`HostInputStream`] that reads to a `Read` type.
///
/// See [`PipedWriteStream`] for more information on why this is synchronous.
pub struct PipeReadStream<T> {
    buffer: Vec<u8>,
    inner: Arc<Mutex<T>>,
}

impl<T> PipeReadStream<T> {
    pub fn new(inner: T) -> Self {
        Self {
            buffer: vec![0_u8; 64 * 1024],
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

impl<T> Clone for PipeReadStream<T> {
    fn clone(&self) -> Self {
        Self {
            buffer: vec![0_u8; 64 * 1024],
            inner: self.inner.clone(),
        }
    }
}

impl<T: Read + Send + Sync + 'static> HostInputStream for PipeReadStream<T> {
    fn read(&mut self, size: usize) -> wasmtime_wasi::StreamResult<bytes::Bytes> {
        let size = size.min(self.buffer.len());

        let count = self
            .inner
            .lock()
            .unwrap()
            .read(&mut self.buffer[..size])
            .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))?;

        Ok(bytes::Bytes::copy_from_slice(&self.buffer[..count]))
    }
}

#[async_trait]
impl<T: Read + Send + Sync + 'static> Subscribe for PipeReadStream<T> {
    async fn ready(&mut self) {}
}

impl<T: Read + Send + Sync + 'static> StdinStream for PipeReadStream<T> {
    fn stream(&self) -> Box<dyn HostInputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}
