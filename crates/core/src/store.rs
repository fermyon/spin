use anyhow::{anyhow, ensure, Result};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};
use system_interface::io::ReadReady;
use tokio::io::{AsyncRead, AsyncWrite};
use wasi_common_preview1 as wasi_preview1;
use wasmtime_wasi as wasmtime_wasi_preview1;
use wasmtime_wasi::preview2::{
    self as wasi_preview2, HostInputStream, HostOutputStream, StdinStream, StdoutStream,
};
use wasmtime_wasi_http::types::WasiHttpCtx;

use crate::{
    host_component::{HostComponents, HostComponentsData},
    io::OutputBuffer,
    limits::StoreLimitsAsync,
    preview1, Data,
};

#[cfg(doc)]
use crate::EngineBuilder;

/// Wrapper for the Preview 1 and Preview 2 versions of `WasiCtx`.
///
/// Currently, only WAGI uses Preview 1, while everything else uses Preview 2 (possibly via an adapter).  WAGI is
/// stuck on Preview 1 and modules because there's no reliable way to wrap an arbitrary Preview 1 command in a
/// component -- the Preview 1 -> 2 adapter only works with modules that either export `canonical_abi_realloc`
/// (e.g. native Spin apps) or use a recent version of `wasi-sdk`, which contains patches to allow the adapter to
/// safely allocate memory via `memory.grow`.
///
/// In theory, someone could build a WAGI app using a new-enough version of `wasi-sdk` and wrap it in a component
/// using the adapter, but that wouldn't add any value beyond leaving it as a module, and any toolchain capable of
/// natively producing components will be capable enough to produce native Spin apps, so we probably won't ever
/// support WAGI components.
///
// TODO: As of this writing, the plan is to merge the WASI Preview 1 and Preview 2 implementations together, at
// which point we'll be able to avoid all the duplication here and below.
pub enum Wasi {
    /// Preview 1 `WasiCtx`
    Preview1(wasi_preview1::WasiCtx),
    /// Preview 2 `WasiCtx`
    Preview2 {
        /// `wasi-cli` context
        wasi_ctx: wasi_preview2::WasiCtx,

        /// `wasi-http` context
        wasi_http_ctx: WasiHttpCtx,
    },
}

/// The version of Wasi being used
#[allow(missing_docs)]
pub enum WasiVersion {
    Preview1,
    Preview2,
}

/// A `Store` holds the runtime state of a Spin instance.
///
/// In general, a `Store` is expected to live only for the lifetime of a single
/// Spin trigger invocation.
///
/// A `Store` can be built with a [`StoreBuilder`].
pub struct Store<T> {
    inner: wasmtime::Store<Data<T>>,
}

impl<T> Store<T> {
    /// Returns a mutable reference to the [`HostComponentsData`] of this [`Store`].
    pub fn host_components_data(&mut self) -> &mut HostComponentsData {
        &mut self.inner.data_mut().host_components_data
    }
}

impl<T> AsRef<wasmtime::Store<Data<T>>> for Store<T> {
    fn as_ref(&self) -> &wasmtime::Store<Data<T>> {
        &self.inner
    }
}

impl<T> AsMut<wasmtime::Store<Data<T>>> for Store<T> {
    fn as_mut(&mut self) -> &mut wasmtime::Store<Data<T>> {
        &mut self.inner
    }
}

impl<T> wasmtime::AsContext for Store<T> {
    type Data = Data<T>;

    fn as_context(&self) -> wasmtime::StoreContext<'_, Self::Data> {
        self.inner.as_context()
    }
}

impl<T> wasmtime::AsContextMut for Store<T> {
    fn as_context_mut(&mut self) -> wasmtime::StoreContextMut<'_, Self::Data> {
        self.inner.as_context_mut()
    }
}

/// A builder interface for configuring a new [`Store`].
///
/// A new [`StoreBuilder`] can be obtained with [`crate::Engine::store_builder`].
pub struct StoreBuilder {
    engine: wasmtime::Engine,
    epoch_tick_interval: Duration,
    yield_interval: Duration,
    wasi: std::result::Result<WasiCtxBuilder, String>,
    host_components_data: HostComponentsData,
    store_limits: StoreLimitsAsync,
}

impl StoreBuilder {
    // Called by Engine::store_builder.
    pub(crate) fn new(
        engine: wasmtime::Engine,
        epoch_tick_interval: Duration,
        host_components: &HostComponents,
        wasi: WasiVersion,
    ) -> Self {
        Self {
            engine,
            epoch_tick_interval,
            yield_interval: epoch_tick_interval,
            wasi: Ok(wasi.into()),
            host_components_data: host_components.new_data(),
            store_limits: StoreLimitsAsync::default(),
        }
    }

    /// Sets a maximum memory allocation limit.
    ///
    /// See [`wasmtime::ResourceLimiter::memory_growing`] (`maximum`) for
    /// details on how this limit is enforced.
    pub fn max_memory_size(&mut self, max_memory_size: usize) {
        self.store_limits = StoreLimitsAsync::new(Some(max_memory_size), None);
    }

    /// Sets the execution yield interval.
    ///
    /// A CPU-bound running instance will be forced to yield approximately
    /// every interval, which gives the host thread an opportunity to cancel
    /// the instance or schedule other work on the thread.
    ///
    /// The exact interval of yielding is determined by [`EngineBuilder::epoch_tick_interval`]
    /// and details of the task scheduler.
    ///
    /// The interval defaults to the epoch tick interval.
    pub fn yield_interval(&mut self, interval: Duration) {
        self.yield_interval = interval;
    }

    /// Inherit stdin from the host process.
    pub fn inherit_stdin(&mut self) {
        self.with_wasi(|wasi| match wasi {
            WasiCtxBuilder::Preview1(ctx) => {
                ctx.set_stdin(Box::new(wasmtime_wasi_preview1::stdio::stdin()))
            }
            WasiCtxBuilder::Preview2(ctx) => {
                ctx.inherit_stdin();
            }
        });
    }

    /// Sets the WASI `stdin` descriptor to the given [`Read`]er.
    pub fn stdin_pipe(
        &mut self,
        r: impl AsyncRead + Read + ReadReady + Send + Sync + Unpin + 'static,
    ) {
        self.with_wasi(|wasi| match wasi {
            WasiCtxBuilder::Preview1(ctx) => {
                ctx.set_stdin(Box::new(wasi_preview1::pipe::ReadPipe::new(r)))
            }
            WasiCtxBuilder::Preview2(ctx) => {
                ctx.stdin(MyStdinStream(Mutex::new(Some(Box::new(
                    wasi_preview2::pipe::AsyncReadStream::new(r),
                )))));
            }
        })
    }

    /// Inherit stdin from the host process.
    pub fn inherit_stdout(&mut self) {
        self.with_wasi(|wasi| match wasi {
            WasiCtxBuilder::Preview1(ctx) => {
                ctx.set_stdout(Box::new(wasmtime_wasi_preview1::stdio::stdout()))
            }
            WasiCtxBuilder::Preview2(ctx) => {
                ctx.inherit_stdout();
            }
        });
    }

    /// Sets the WASI `stdout` descriptor to the given [`Write`]er.
    pub fn stdout(&mut self, w: Box<dyn wasi_preview1::WasiFile>) -> Result<()> {
        self.try_with_wasi(|wasi| match wasi {
            WasiCtxBuilder::Preview1(ctx) => {
                ctx.set_stdout(w);
                Ok(())
            }
            WasiCtxBuilder::Preview2(_) => Err(anyhow!(
                "`Store::stdout` only supported with WASI Preview 1"
            )),
        })
    }

    /// Sets the WASI `stdout` descriptor to the given [`Write`]er.
    pub fn stdout_pipe(&mut self, w: impl AsyncWrite + Write + Send + Sync + Unpin + 'static) {
        self.with_wasi(|wasi| match wasi {
            WasiCtxBuilder::Preview1(ctx) => {
                ctx.set_stdout(Box::new(wasi_preview1::pipe::WritePipe::new(w)))
            }
            WasiCtxBuilder::Preview2(ctx) => {
                ctx.stdout(MyStdoutStream(Mutex::new(Some(Box::new(
                    wasi_preview2::pipe::AsyncWriteStream::new(1024 * 1024, w),
                )))));
            }
        })
    }

    /// Sets the WASI `stdout` descriptor to an in-memory buffer which can be
    /// retrieved after execution from the returned [`OutputBuffer`].
    pub fn stdout_buffered(&mut self) -> Result<OutputBuffer> {
        let buffer = OutputBuffer::default();
        // This only needs to work with Preview 2 since WAGI does its own thing with Preview 1:
        self.try_with_wasi(|wasi| match wasi {
            WasiCtxBuilder::Preview1(_) => Err(anyhow!(
                "`Store::stdout_buffered` only supported with WASI Preview 2"
            )),
            WasiCtxBuilder::Preview2(ctx) => {
                ctx.stdout(MyStdoutStream(Mutex::new(Some(Box::new(buffer.writer())))));
                Ok(())
            }
        })?;
        Ok(buffer)
    }

    /// Inherit stdin from the host process.
    pub fn inherit_stderr(&mut self) {
        self.with_wasi(|wasi| match wasi {
            WasiCtxBuilder::Preview1(ctx) => {
                ctx.set_stderr(Box::new(wasmtime_wasi_preview1::stdio::stderr()))
            }
            WasiCtxBuilder::Preview2(ctx) => {
                ctx.inherit_stderr();
            }
        });
    }

    /// Sets the WASI `stderr` descriptor to the given [`Write`]er.
    pub fn stderr_pipe(&mut self, w: impl AsyncWrite + Write + Send + Sync + Unpin + 'static) {
        self.with_wasi(|wasi| match wasi {
            WasiCtxBuilder::Preview1(ctx) => {
                ctx.set_stderr(Box::new(wasi_preview1::pipe::WritePipe::new(w)))
            }
            WasiCtxBuilder::Preview2(ctx) => {
                ctx.stderr(MyStdoutStream(Mutex::new(Some(Box::new(
                    wasi_preview2::pipe::AsyncWriteStream::new(1024 * 1024, w),
                )))));
            }
        })
    }

    /// Appends the given strings to the the WASI 'args'.
    pub fn args<'b>(&mut self, args: impl IntoIterator<Item = &'b str>) -> Result<()> {
        self.try_with_wasi(|wasi| {
            for arg in args {
                match wasi {
                    WasiCtxBuilder::Preview1(ctx) => ctx.push_arg(arg)?,
                    WasiCtxBuilder::Preview2(ctx) => {
                        ctx.arg(arg);
                    }
                }
            }
            Ok(())
        })
    }

    /// Sets the given key/value string entries on the the WASI 'env'.
    pub fn env(
        &mut self,
        vars: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Result<()> {
        self.try_with_wasi(|wasi| {
            for (k, v) in vars {
                match wasi {
                    WasiCtxBuilder::Preview1(ctx) => ctx.push_env(k.as_ref(), v.as_ref())?,
                    WasiCtxBuilder::Preview2(ctx) => {
                        ctx.env(k, v);
                    }
                }
            }

            Ok(())
        })
    }

    /// "Mounts" the given `host_path` into the WASI filesystem at the given
    /// `guest_path` with read-only capabilities.
    pub fn read_only_preopened_dir(
        &mut self,
        host_path: impl AsRef<Path>,
        guest_path: PathBuf,
    ) -> Result<()> {
        self.preopened_dir_impl(host_path, guest_path, false)
    }

    /// "Mounts" the given `host_path` into the WASI filesystem at the given
    /// `guest_path` with read and write capabilities.
    pub fn read_write_preopened_dir(
        &mut self,
        host_path: impl AsRef<Path>,
        guest_path: PathBuf,
    ) -> Result<()> {
        self.preopened_dir_impl(host_path, guest_path, true)
    }

    fn preopened_dir_impl(
        &mut self,
        host_path: impl AsRef<Path>,
        guest_path: PathBuf,
        writable: bool,
    ) -> Result<()> {
        let cap_std_dir =
            cap_std::fs::Dir::open_ambient_dir(host_path.as_ref(), cap_std::ambient_authority())?;
        let path = guest_path
            .to_str()
            .ok_or_else(|| anyhow!("non-utf8 path: {}", guest_path.display()))?;

        self.try_with_wasi(|wasi| {
            match wasi {
                WasiCtxBuilder::Preview1(ctx) => {
                    let mut dir =
                        Box::new(wasmtime_wasi_preview1::dir::Dir::from_cap_std(cap_std_dir)) as _;
                    if !writable {
                        dir = Box::new(preview1::ReadOnlyDir(dir));
                    }
                    ctx.push_preopened_dir(dir, path)?;
                }
                WasiCtxBuilder::Preview2(ctx) => {
                    let dir_perms = if writable {
                        wasi_preview2::DirPerms::all()
                    } else {
                        wasi_preview2::DirPerms::READ
                    };
                    let file_perms = wasi_preview2::FilePerms::all();

                    ctx.preopened_dir(cap_std_dir, dir_perms, file_perms, path);
                }
            }
            Ok(())
        })
    }

    /// Returns a mutable reference to the built
    pub fn host_components_data(&mut self) -> &mut HostComponentsData {
        &mut self.host_components_data
    }

    /// Builds a [`Store`] from this builder with given host state data.
    ///
    /// If `T: Default`, it may be preferable to use [`Store::build`].
    pub fn build_with_data<T>(self, inner_data: T) -> Result<Store<T>> {
        let wasi = self.wasi.map_err(anyhow::Error::msg)?.build();

        let mut inner = wasmtime::Store::new(
            &self.engine,
            Data {
                inner: inner_data,
                wasi,
                host_components_data: self.host_components_data,
                store_limits: self.store_limits,
                table: wasi_preview2::Table::new(),
            },
        );

        inner.limiter_async(move |data| &mut data.store_limits);

        ensure!(
            !self.epoch_tick_interval.is_zero(),
            "epoch_tick_interval may not be zero"
        );
        let delta = self.yield_interval.as_nanos() / self.epoch_tick_interval.as_nanos();
        let delta = if delta == 0 {
            tracing::warn!(
                "Yield interval {interval:?} too small to resolve; clamping to tick interval {tick:?}",
                interval = self.yield_interval,
                tick = self.epoch_tick_interval);
            1
        } else if delta > u64::MAX as u128 {
            tracing::warn!("Yield interval too large; yielding effectively disabled");
            u64::MAX
        } else {
            delta as u64
        };
        inner.epoch_deadline_async_yield_and_update(delta);

        Ok(Store { inner })
    }

    /// Builds a [`Store`] from this builder with `Default` host state data.
    pub fn build<T: Default>(self) -> Result<Store<T>> {
        self.build_with_data(T::default())
    }

    fn with_wasi(&mut self, f: impl FnOnce(&mut WasiCtxBuilder)) {
        let _ = self.try_with_wasi(|wasi| {
            f(wasi);
            Ok(())
        });
    }

    fn try_with_wasi(&mut self, f: impl FnOnce(&mut WasiCtxBuilder) -> Result<()>) -> Result<()> {
        let wasi = self
            .wasi
            .as_mut()
            .map_err(|err| anyhow!("StoreBuilder already failed: {}", err))?;

        match f(wasi) {
            Ok(()) => Ok(()),
            Err(err) => {
                self.wasi = Err(err.to_string());
                Err(err)
            }
        }
    }
}

struct MyStdinStream(Mutex<Option<Box<dyn HostInputStream>>>);

impl StdinStream for MyStdinStream {
    fn stream(&self) -> Box<dyn HostInputStream> {
        self.0
            .lock()
            .unwrap()
            .take()
            .expect("MyStdinStream::stream should only be called once")
    }

    fn isatty(&self) -> bool {
        false
    }
}

struct MyStdoutStream(Mutex<Option<Box<dyn HostOutputStream>>>);

impl StdoutStream for MyStdoutStream {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        self.0
            .lock()
            .unwrap()
            .take()
            .expect("MyStdoutStream::stream should only be called once")
    }

    fn isatty(&self) -> bool {
        false
    }
}

/// A builder of a `WasiCtx` for all versions of Wasi
#[allow(clippy::large_enum_variant)]
enum WasiCtxBuilder {
    Preview1(wasi_preview1::WasiCtx),
    Preview2(wasi_preview2::WasiCtxBuilder),
}

impl From<WasiVersion> for WasiCtxBuilder {
    fn from(value: WasiVersion) -> Self {
        match value {
            WasiVersion::Preview1 => {
                Self::Preview1(wasmtime_wasi_preview1::WasiCtxBuilder::new().build())
            }
            WasiVersion::Preview2 => Self::Preview2(wasi_preview2::WasiCtxBuilder::new()),
        }
    }
}

impl WasiCtxBuilder {
    fn build(self) -> Wasi {
        match self {
            WasiCtxBuilder::Preview1(ctx) => Wasi::Preview1(ctx),
            WasiCtxBuilder::Preview2(mut b) => Wasi::Preview2 {
                wasi_ctx: b.build(),
                wasi_http_ctx: WasiHttpCtx,
            },
        }
    }
}
