mod host_component;
mod io;
mod limits;
mod store;

use std::sync::{Arc, Mutex};

use anyhow::Result;
use tracing::instrument;
use wasmtime_wasi::WasiCtx;

pub use wasmtime::{self, Instance, Module};

use self::host_component::{HostComponents, HostComponentsBuilder};

pub use host_component::{HostComponent, HostComponentDataHandle, HostComponentsData};
pub use store::{Store, StoreBuilder};

pub struct Config {
    inner: wasmtime::Config,
}

impl Config {
    /// Borrow the inner wasmtime::Config mutably.
    /// WARNING: This is inherently unstable and may break at any time!
    #[doc(hidden)]
    pub fn wasmtime_config(&mut self) -> &mut wasmtime::Config {
        &mut self.inner
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut inner = wasmtime::Config::new();
        inner.async_support(true);
        Self { inner }
    }
}

pub struct Data<T> {
    inner: T,
    wasi: WasiCtx,
    host_components_data: HostComponentsData,
    store_limits: limits::StoreLimitsAsync,
}

impl<T> AsRef<T> for Data<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> AsMut<T> for Data<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

pub type Linker<T> = wasmtime::Linker<Data<T>>;

pub struct EngineBuilder<T> {
    engine: wasmtime::Engine,
    linker: Linker<T>,
    host_components_builder: HostComponentsBuilder,
}

impl<T: Send + Sync> EngineBuilder<T> {
    fn new(config: &Config) -> Result<Self> {
        let engine = wasmtime::Engine::new(&config.inner)?;

        let mut linker: Linker<T> = Linker::new(&engine);
        wasmtime_wasi::tokio::add_to_linker(&mut linker, |data| &mut data.wasi)?;

        Ok(Self {
            engine,
            linker,
            host_components_builder: HostComponents::builder(),
        })
    }

    pub fn link_import(
        &mut self,
        f: impl FnOnce(&mut Linker<T>, fn(&mut Data<T>) -> &mut T) -> Result<()>,
    ) -> Result<()> {
        f(&mut self.linker, Data::as_mut)
    }

    pub fn add_host_component<HC: HostComponent + Send + Sync + 'static>(
        &mut self,
        host_component: HC,
    ) -> Result<HostComponentDataHandle<HC>> {
        self.host_components_builder
            .add_host_component(&mut self.linker, host_component)
    }

    pub fn build_with_data(self, instance_pre_data: T) -> Engine<T> {
        let host_components = self.host_components_builder.build();

        let instance_pre_store = Arc::new(Mutex::new(
            StoreBuilder::new(self.engine.clone(), &host_components)
                .build_with_data(instance_pre_data)
                .expect("instance_pre_store build should not fail"),
        ));

        Engine {
            inner: self.engine,
            linker: self.linker,
            host_components,
            instance_pre_store,
        }
    }
}

impl<T: Default + Send + Sync> EngineBuilder<T> {
    pub fn build(self) -> Engine<T> {
        self.build_with_data(T::default())
    }
}

pub struct Engine<T> {
    inner: wasmtime::Engine,
    linker: Linker<T>,
    host_components: HostComponents,
    instance_pre_store: Arc<Mutex<Store<T>>>,
}

impl<T: Send + Sync> Engine<T> {
    pub fn builder(config: &Config) -> Result<EngineBuilder<T>> {
        EngineBuilder::new(config)
    }

    pub fn store_builder(&self) -> StoreBuilder {
        StoreBuilder::new(self.inner.clone(), &self.host_components)
    }

    #[instrument(skip_all)]
    pub fn instantiate_pre(&self, module: &Module) -> Result<InstancePre<T>> {
        let mut store = self.instance_pre_store.lock().unwrap();
        let inner = self.linker.instantiate_pre(&mut *store, module)?;
        Ok(InstancePre { inner })
    }
}

impl<T> AsRef<wasmtime::Engine> for Engine<T> {
    fn as_ref(&self) -> &wasmtime::Engine {
        &self.inner
    }
}

pub struct InstancePre<T> {
    inner: wasmtime::InstancePre<Data<T>>,
}

impl<T: Send + Sync> InstancePre<T> {
    #[instrument(skip_all)]
    pub async fn instantiate_async(&self, store: &mut Store<T>) -> Result<Instance> {
        self.inner.instantiate_async(store).await
    }
}

impl<T> Clone for InstancePre<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> AsRef<wasmtime::InstancePre<Data<T>>> for InstancePre<T> {
    fn as_ref(&self) -> &wasmtime::InstancePre<Data<T>> {
        &self.inner
    }
}
