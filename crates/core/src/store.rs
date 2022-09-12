use anyhow::{anyhow, Result};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};
use wasi_cap_std_sync::{ambient_authority, Dir};
use wasi_common::{dir::DirCaps, pipe::WritePipe, WasiFile};
use wasi_common::{file::FileCaps, pipe::ReadPipe};
use wasmtime_wasi::tokio::WasiCtxBuilder;

use crate::io::OutputBuffer;

use super::{
    host_component::{HostComponents, HostComponentsData},
    limits::StoreLimitsAsync,
    Data,
};

pub struct Store<T> {
    inner: wasmtime::Store<Data<T>>,
}

impl<T> Store<T> {
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

pub struct StoreBuilder {
    engine: wasmtime::Engine,
    wasi: std::result::Result<Option<WasiCtxBuilder>, String>,
    read_only_preopened_dirs: Vec<(Dir, PathBuf)>,
    host_components_data: HostComponentsData,
    store_limits: StoreLimitsAsync,
}

impl StoreBuilder {
    pub(crate) fn new(engine: wasmtime::Engine, host_components: &HostComponents) -> Self {
        Self {
            engine,
            wasi: Ok(Some(WasiCtxBuilder::new())),
            read_only_preopened_dirs: Vec::new(),
            host_components_data: host_components.new_data(),
            store_limits: StoreLimitsAsync::default(),
        }
    }

    pub fn max_memory_size(&mut self, max_memory_size: usize) {
        self.store_limits = StoreLimitsAsync::new(Some(max_memory_size), None);
    }

    pub fn inherit_stdio(&mut self) {
        self.with_wasi(|wasi| wasi.inherit_stdio());
    }

    pub fn stdin(&mut self, file: impl WasiFile + 'static) {
        self.with_wasi(|wasi| wasi.stdin(Box::new(file)))
    }

    pub fn stdin_pipe(&mut self, r: impl Read + Send + Sync + 'static) {
        self.stdin(ReadPipe::new(r))
    }

    pub fn stdout(&mut self, file: impl WasiFile + 'static) {
        self.with_wasi(|wasi| wasi.stdout(Box::new(file)))
    }

    pub fn stdout_pipe(&mut self, w: impl Write + Send + Sync + 'static) {
        self.stdout(WritePipe::new(w))
    }

    pub fn stdout_buffered(&mut self) -> OutputBuffer {
        let buffer = OutputBuffer::default();
        self.stdout(buffer.writer());
        buffer
    }

    pub fn stderr(&mut self, file: impl WasiFile + 'static) {
        self.with_wasi(|wasi| wasi.stderr(Box::new(file)))
    }

    pub fn stderr_pipe(&mut self, w: impl Write + Send + Sync + 'static) {
        self.stderr(WritePipe::new(w))
    }

    pub fn stderr_buffered(&mut self) -> OutputBuffer {
        let buffer = OutputBuffer::default();
        self.stderr(buffer.writer());
        buffer
    }

    pub fn args<'b>(&mut self, args: impl IntoIterator<Item = &'b str>) -> Result<()> {
        self.try_with_wasi(|mut wasi| {
            for arg in args {
                wasi = wasi.arg(arg)?;
            }
            Ok(wasi)
        })
    }

    pub fn env(
        &mut self,
        vars: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Result<()> {
        self.try_with_wasi(|mut wasi| {
            for (k, v) in vars {
                wasi = wasi.env(k.as_ref(), v.as_ref())?;
            }
            Ok(wasi)
        })
    }

    pub fn read_only_preopened_dir(
        &mut self,
        host_path: impl AsRef<Path>,
        guest_path: PathBuf,
    ) -> Result<()> {
        // WasiCtxBuilder::preopened_dir doesn't let you set capabilities, so we need
        // to wait and call WasiCtx::insert_dir after building the WasiCtx.
        let dir = wasmtime_wasi::Dir::open_ambient_dir(host_path, ambient_authority())?;
        self.read_only_preopened_dirs.push((dir, guest_path));
        Ok(())
    }

    pub fn read_write_preopened_dir(
        &mut self,
        host_path: impl AsRef<Path>,
        guest_path: PathBuf,
    ) -> Result<()> {
        let dir = wasmtime_wasi::Dir::open_ambient_dir(host_path, ambient_authority())?;
        self.try_with_wasi(|wasi| wasi.preopened_dir(dir, guest_path))
    }

    pub fn host_components_data(&mut self) -> &mut HostComponentsData {
        &mut self.host_components_data
    }

    pub fn build_with_data<T>(self, inner_data: T) -> Result<Store<T>> {
        let mut wasi = self.wasi.map_err(anyhow::Error::msg)?.unwrap().build();

        // Insert any read-only preopened dirs
        for (idx, (dir, path)) in self.read_only_preopened_dirs.into_iter().enumerate() {
            let fd = WASI_FIRST_PREOPENED_DIR_FD + idx as u32;
            let dir = Box::new(wasmtime_wasi::tokio::Dir::from_cap_std(dir));
            wasi.insert_dir(fd, dir, READ_ONLY_DIR_CAPS, READ_ONLY_FILE_CAPS, path);
        }

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
        Ok(Store { inner })
    }

    pub fn build<T: Default>(self) -> Result<Store<T>> {
        self.build_with_data(T::default())
    }

    fn with_wasi(&mut self, f: impl FnOnce(WasiCtxBuilder) -> WasiCtxBuilder) {
        let _ = self.try_with_wasi(|wasi| Ok(f(wasi)));
    }

    fn try_with_wasi(
        &mut self,
        f: impl FnOnce(WasiCtxBuilder) -> Result<WasiCtxBuilder>,
    ) -> Result<()> {
        let wasi = self
            .wasi
            .as_mut()
            .map_err(|err| anyhow!("StoreBuilder already failed: {}", err))?
            .take()
            .unwrap();
        match f(wasi) {
            Ok(wasi) => {
                self.wasi = Ok(Some(wasi));
                Ok(())
            }
            Err(err) => {
                self.wasi = Err(err.to_string());
                Err(err)
            }
        }
    }
}
