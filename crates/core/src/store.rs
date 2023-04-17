use anyhow::{anyhow, Result};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use system_interface::io::ReadReady;
use wasi_cap_std_sync as wasmtime_wasi_preview2;
use wasi_common as wasi_preview2;
use wasi_common_preview1::{self as wasi_preview1, dir::DirCaps, file::FileCaps};
use wasmtime_wasi as wasmtime_wasi_preview1;

use crate::{
    host_component::{HostComponents, HostComponentsData},
    io::OutputBuffer,
    limits::StoreLimitsAsync,
    Data,
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
    Preview2(wasi_preview2::WasiCtx),
}

impl Wasi {
    /// Create a new `Wasi::Preview1` context
    pub fn new_preview1() -> Self {
        Self::Preview1(wasmtime_wasi_preview1::WasiCtxBuilder::new().build())
    }

    /// Create a new `Wasi::Preview2` context
    pub fn new_preview2() -> Self {
        Self::Preview2(wasmtime_wasi_preview2::WasiCtxBuilder::new().build())
    }
}

/// A `Store` holds the runtime state of a Spin instance.
///
/// In general, a `Store` is expected to live only for the lifetime of a single
/// Spin trigger invocation.
///
/// A `Store` can be built with a [`StoreBuilder`].
pub struct Store<T> {
    inner: wasmtime::Store<Data<T>>,
    epoch_tick_interval: Duration,
}

impl<T> Store<T> {
    /// Returns a mutable reference to the [`HostComponentsData`] of this [`Store`].
    pub fn host_components_data(&mut self) -> &mut HostComponentsData {
        &mut self.inner.data_mut().host_components_data
    }

    /// Sets the execution deadline.
    ///
    /// This is a rough deadline; an instance will trap some time after this
    /// deadline, determined by [`EngineBuilder::epoch_tick_interval`] and
    /// details of the system's thread scheduler.
    ///
    /// See [`wasmtime::Store::set_epoch_deadline`](https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.set_epoch_deadline).
    pub fn set_deadline(&mut self, deadline: Instant) {
        let now = Instant::now();
        let duration = deadline - now;
        let ticks = if duration.is_zero() {
            tracing::warn!("Execution deadline set in past: {deadline:?} < {now:?}");
            0
        } else {
            let ticks = duration.as_micros() / self.epoch_tick_interval.as_micros();
            let ticks = ticks.min(u64::MAX as u128) as u64;
            ticks + 1 // Add one to allow for current partially-completed tick
        };
        self.inner.set_epoch_deadline(ticks);
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

// WASI expects preopened dirs in FDs starting at 3 (0-2 are stdio).
const WASI_FIRST_PREOPENED_DIR_FD: u32 = 3;

const READ_ONLY_DIR_CAPS: DirCaps = DirCaps::from_bits_truncate(
    DirCaps::OPEN.bits()
        | DirCaps::READDIR.bits()
        | DirCaps::READLINK.bits()
        | DirCaps::PATH_FILESTAT_GET.bits()
        | DirCaps::FILESTAT_GET.bits(),
);
const READ_ONLY_FILE_CAPS: FileCaps = FileCaps::from_bits_truncate(
    FileCaps::READ.bits()
        | FileCaps::SEEK.bits()
        | FileCaps::TELL.bits()
        | FileCaps::FILESTAT_GET.bits()
        | FileCaps::POLL_READWRITE.bits(),
);

/// A builder interface for configuring a new [`Store`].
///
/// A new [`StoreBuilder`] can be obtained with [`crate::Engine::store_builder`].
pub struct StoreBuilder {
    engine: wasmtime::Engine,
    epoch_tick_interval: Duration,
    wasi: std::result::Result<Wasi, String>,
    host_components_data: HostComponentsData,
    store_limits: StoreLimitsAsync,
    next_preopen_index: u32,
}

impl StoreBuilder {
    // Called by Engine::store_builder.
    pub(crate) fn new(
        engine: wasmtime::Engine,
        epoch_tick_interval: Duration,
        host_components: &HostComponents,
        wasi: Wasi,
    ) -> Self {
        Self {
            engine,
            epoch_tick_interval,
            wasi: Ok(wasi),
            host_components_data: host_components.new_data(),
            store_limits: StoreLimitsAsync::default(),
            next_preopen_index: WASI_FIRST_PREOPENED_DIR_FD,
        }
    }

    /// Sets a maximum memory allocation limit.
    ///
    /// See [`wasmtime::ResourceLimiter::memory_growing`] (`maximum`) for
    /// details on how this limit is enforced.
    pub fn max_memory_size(&mut self, max_memory_size: usize) {
        self.store_limits = StoreLimitsAsync::new(Some(max_memory_size), None);
    }

    /// Inherit stdin from the host process.
    pub fn inherit_stdin(&mut self) {
        self.with_wasi(|wasi| match wasi {
            Wasi::Preview1(ctx) => ctx.set_stdin(Box::new(wasmtime_wasi_preview1::stdio::stdin())),
            Wasi::Preview2(ctx) => ctx.set_stdin(Box::new(wasmtime_wasi_preview2::stdio::stdin())),
        });
    }

    /// Sets the WASI `stdin` descriptor to the given [`Read`]er.
    pub fn stdin_pipe(&mut self, r: impl Read + ReadReady + Send + Sync + 'static) {
        self.with_wasi(|wasi| match wasi {
            Wasi::Preview1(ctx) => ctx.set_stdin(Box::new(wasi_preview1::pipe::ReadPipe::new(r))),
            Wasi::Preview2(ctx) => ctx.set_stdin(Box::new(wasi_preview2::pipe::ReadPipe::new(r))),
        })
    }

    /// Inherit stdin from the host process.
    pub fn inherit_stdout(&mut self) {
        self.with_wasi(|wasi| match wasi {
            Wasi::Preview1(ctx) => {
                ctx.set_stdout(Box::new(wasmtime_wasi_preview1::stdio::stdout()))
            }
            Wasi::Preview2(ctx) => {
                ctx.set_stdout(Box::new(wasmtime_wasi_preview2::stdio::stdout()))
            }
        });
    }

    /// Sets the WASI `stdout` descriptor to the given [`Write`]er.
    pub fn stdout(&mut self, w: Box<dyn wasi_preview1::WasiFile>) -> Result<()> {
        self.try_with_wasi(|wasi| match wasi {
            Wasi::Preview1(ctx) => {
                ctx.set_stdout(w);
                Ok(())
            }
            Wasi::Preview2(_) => Err(anyhow!(
                "`Store::stdout` only supported with WASI Preview 1"
            )),
        })
    }

    /// Sets the WASI `stdout` descriptor to the given [`Write`]er.
    pub fn stdout_pipe(&mut self, w: impl Write + Send + Sync + 'static) {
        self.with_wasi(|wasi| match wasi {
            Wasi::Preview1(ctx) => ctx.set_stdout(Box::new(wasi_preview1::pipe::WritePipe::new(w))),
            Wasi::Preview2(ctx) => ctx.set_stdout(Box::new(wasi_preview2::pipe::WritePipe::new(w))),
        })
    }

    /// Sets the WASI `stdout` descriptor to an in-memory buffer which can be
    /// retrieved after execution from the returned [`OutputBuffer`].
    pub fn stdout_buffered(&mut self) -> Result<OutputBuffer> {
        let buffer = OutputBuffer::default();
        // This only needs to work with Preview 2 since WAGI does its own thing with Preview 1:
        self.try_with_wasi(|wasi| match wasi {
            Wasi::Preview1(_) => Err(anyhow!(
                "`Store::stdout_buffered` only supported with WASI Preview 2"
            )),
            Wasi::Preview2(ctx) => {
                ctx.set_stdout(Box::new(buffer.writer()));
                Ok(())
            }
        })?;
        Ok(buffer)
    }

    /// Inherit stdin from the host process.
    pub fn inherit_stderr(&mut self) {
        self.with_wasi(|wasi| match wasi {
            Wasi::Preview1(ctx) => {
                ctx.set_stderr(Box::new(wasmtime_wasi_preview1::stdio::stderr()))
            }
            Wasi::Preview2(ctx) => {
                ctx.set_stderr(Box::new(wasmtime_wasi_preview2::stdio::stderr()))
            }
        });
    }

    /// Sets the WASI `stderr` descriptor to the given [`Write`]er.
    pub fn stderr_pipe(&mut self, w: impl Write + Send + Sync + 'static) {
        self.with_wasi(|wasi| match wasi {
            Wasi::Preview1(ctx) => ctx.set_stderr(Box::new(wasi_preview1::pipe::WritePipe::new(w))),
            Wasi::Preview2(ctx) => ctx.set_stderr(Box::new(wasi_preview2::pipe::WritePipe::new(w))),
        })
    }

    /// Appends the given strings to the the WASI 'args'.
    pub fn args<'b>(&mut self, args: impl IntoIterator<Item = &'b str>) -> Result<()> {
        self.try_with_wasi(|wasi| {
            for arg in args {
                match wasi {
                    Wasi::Preview1(ctx) => ctx.push_arg(arg)?,
                    Wasi::Preview2(ctx) => ctx.push_arg(arg),
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
                    Wasi::Preview1(ctx) => ctx.push_env(k.as_ref(), v.as_ref())?,
                    Wasi::Preview2(ctx) => ctx.push_env(k.as_ref(), v.as_ref()),
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
        let dir =
            || cap_std::fs::Dir::open_ambient_dir(host_path.as_ref(), cap_std::ambient_authority());
        let path = guest_path
            .to_str()
            .ok_or_else(|| anyhow!("non-utf8 path: {}", guest_path.display()))?;
        let index = self.next_preopen_index;

        self.try_with_wasi(|wasi| {
            match wasi {
                Wasi::Preview1(ctx) => ctx.insert_dir(
                    index,
                    Box::new(wasmtime_wasi_preview1::dir::Dir::from_cap_std(dir()?)),
                    READ_ONLY_DIR_CAPS,
                    READ_ONLY_FILE_CAPS,
                    path.into(),
                ),
                Wasi::Preview2(ctx) => ctx.push_preopened_dir(
                    Box::new(wasi_preview2::dir::ReadOnlyDir(Box::new(
                        wasmtime_wasi_preview2::dir::Dir::from_cap_std(dir()?),
                    ))),
                    path,
                )?,
            }
            Ok(())
        })?;

        self.next_preopen_index += 1;

        Ok(())
    }

    /// "Mounts" the given `host_path` into the WASI filesystem at the given
    /// `guest_path` with read and write capabilities.
    pub fn read_write_preopened_dir(
        &mut self,
        host_path: impl AsRef<Path>,
        guest_path: PathBuf,
    ) -> Result<()> {
        let dir =
            || cap_std::fs::Dir::open_ambient_dir(host_path.as_ref(), cap_std::ambient_authority());
        let path = guest_path
            .to_str()
            .ok_or_else(|| anyhow!("non-utf8 path: {}", guest_path.display()))?;

        self.try_with_wasi(|wasi| {
            match wasi {
                Wasi::Preview1(ctx) => ctx.push_preopened_dir(
                    Box::new(wasmtime_wasi_preview1::dir::Dir::from_cap_std(dir()?)),
                    path,
                )?,
                Wasi::Preview2(ctx) => ctx.push_preopened_dir(
                    Box::new(wasmtime_wasi_preview2::dir::Dir::from_cap_std(dir()?)),
                    path,
                )?,
            }
            Ok(())
        })?;

        self.next_preopen_index += 1;

        Ok(())
    }

    /// Returns a mutable reference to the built
    pub fn host_components_data(&mut self) -> &mut HostComponentsData {
        &mut self.host_components_data
    }

    /// Builds a [`Store`] from this builder with given host state data.
    ///
    /// If `T: Default`, it may be preferable to use [`Store::build`].
    pub fn build_with_data<T>(self, inner_data: T) -> Result<Store<T>> {
        let wasi = self.wasi.map_err(anyhow::Error::msg)?;

        let mut inner = wasmtime::Store::new(
            &self.engine,
            Data {
                inner: inner_data,
                wasi,
                host_components_data: self.host_components_data,
                store_limits: self.store_limits,
            },
        );

        inner.limiter_async(move |data| &mut data.store_limits);

        // With epoch interruption enabled, there must be _some_ deadline set
        // or execution will trap immediately. Since this is a delta, we need
        // to avoid overflow so we'll use 2^63 which is still "practically
        // forever" for any plausible tick interval.
        inner.set_epoch_deadline(u64::MAX / 2);

        Ok(Store {
            inner,
            epoch_tick_interval: self.epoch_tick_interval,
        })
    }

    /// Builds a [`Store`] from this builder with `Default` host state data.
    pub fn build<T: Default>(self) -> Result<Store<T>> {
        self.build_with_data(T::default())
    }

    fn with_wasi(&mut self, f: impl FnOnce(&mut Wasi)) {
        let _ = self.try_with_wasi(|wasi| {
            f(wasi);
            Ok(())
        });
    }

    fn try_with_wasi(&mut self, f: impl FnOnce(&mut Wasi) -> Result<()>) -> Result<()> {
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
