//! Spin core execution engine
//!
//! This crate provides low-level Wasm and WASI functionality required by Spin.
//! Most of this functionality consists of wrappers around [`wasmtime`] and
//! [`wasmtime_wasi`] that narrows the flexibility of `wasmtime` to the set of
//! features used by Spin (such as only supporting `wasmtime`'s async calling style).

#![deny(missing_docs)]

mod host_component;
mod io;
mod limits;
mod store;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use crossbeam_channel::Sender;
use tracing::instrument;
pub use wasmtime::{self, Instance, Module, Trap};
pub use wasmtime_wasi::I32Exit;
use wasmtime_wasi::WasiCtx;

use self::host_component::{HostComponents, HostComponentsBuilder};

pub use host_component::{HostComponent, HostComponentDataHandle, HostComponentsData};
pub use io::OutputBuffer;
pub use store::{Store, StoreBuilder};

/// The default [`EngineBuilder::epoch_tick_interval`].
pub const DEFAULT_EPOCH_TICK_INTERVAL: Duration = Duration::from_millis(10);

/// The default number of memories an instance can have when using the pooling instance allocator.
pub const DEFAULT_INSTANCE_MEMORIES: u32 = 1;

const MB: u64 = 1 << 20;
const WASM_PAGE_SIZE: u64 = 64 * 1024;

/// The default maximum size of an instance's memories, in 64kb WebAssembly pages.
pub const DEFAULT_INSTANCE_MEMORY_PAGES: u64 = 128 * MB / WASM_PAGE_SIZE;

/// The default number of tables an instance can have when using the pooling instance allocator.
pub const DEFAULT_INSTANCE_TABLES: u32 = 1;

/// The default number of elements an instance's tables can contain when using the pooling instance allocator.
pub const DEFAULT_INSTANCE_TABLE_ELEMENTS: u32 = 100_000;

/// Global configuration for `EngineBuilder`.
///
/// This is currently only used for advanced (undocumented) use cases.
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

    /// Enable the Wasmtime compilation cache with the given path, if any, to load configuration from.
    pub fn configure_cache(&mut self, config_path: &Option<PathBuf>) -> Result<()> {
        match config_path {
            Some(p) => self.inner.cache_config_load(p)?,
            None => self.inner.cache_config_load_default()?,
        };

        Ok(())
    }

    /// Enable or update parameters for the pooling instance allocator.
    pub fn enable_pooling(
        &mut self,
        max_memories: u32,
        max_memory_pages: u64,
        max_tables: u32,
        max_table_entries: u32,
    ) -> &mut Self {
        use wasmtime::{InstanceAllocationStrategy, PoolingAllocationConfig};

        let mut pooling_config = PoolingAllocationConfig::default();
        pooling_config
            .instance_memories(max_memories)
            .instance_memory_pages(max_memory_pages)
            .instance_tables(max_tables)
            // Some wasm modules tend to have very large tables, in particular in non-optimized builds.
            .instance_table_elements(max_table_entries);

        self.inner
            .allocation_strategy(InstanceAllocationStrategy::Pooling(pooling_config));

        self
    }

    /// Disable the pooling instance allocator.
    pub fn disable_pooling(&mut self) -> &mut Self {
        self.inner
            .allocation_strategy(wasmtime::InstanceAllocationStrategy::OnDemand);
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut inner = wasmtime::Config::new();
        inner.async_support(true);
        inner.epoch_interruption(true);

        let mut config = Self { inner };
        config.enable_pooling(
            DEFAULT_INSTANCE_MEMORIES,
            DEFAULT_INSTANCE_MEMORY_PAGES,
            DEFAULT_INSTANCE_TABLES,
            DEFAULT_INSTANCE_TABLE_ELEMENTS,
        );

        config
    }
}

/// Host state data associated with individual [Store]s and [Instance]s.
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

/// An alias for [`wasmtime::Linker`] specialized to [`Data`].
pub type Linker<T> = wasmtime::Linker<Data<T>>;

/// A builder interface for configuring a new [`Engine`].
///
/// A new [`EngineBuilder`] can be obtained with [`Engine::builder`].
pub struct EngineBuilder<T> {
    engine: wasmtime::Engine,
    linker: Linker<T>,
    host_components_builder: HostComponentsBuilder,
    epoch_tick_interval: Duration,
    epoch_ticker_thread: bool,
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
            epoch_tick_interval: DEFAULT_EPOCH_TICK_INTERVAL,
            epoch_ticker_thread: true,
        })
    }

    /// Adds definition(s) to the built [`Engine`].
    ///
    /// This method's signature is meant to be used with
    /// [`wit-bindgen`](https://github.com/bytecodealliance/wit-bindgen)'s
    /// generated `add_to_linker` functions, e.g.:
    ///
    /// ```ignore
    /// wit_bindgen_wasmtime::import!({paths: ["my-interface.wit"], async: *});
    /// // ...
    /// let mut builder: EngineBuilder<my_interface::MyInterfaceData> = Engine::builder();
    /// builder.link_import(my_interface::MyInterface::add_to_linker)?;
    /// ```
    pub fn link_import(
        &mut self,
        f: impl FnOnce(&mut Linker<T>, fn(&mut Data<T>) -> &mut T) -> Result<()>,
    ) -> Result<()> {
        f(&mut self.linker, Data::as_mut)
    }

    /// Adds a [`HostComponent`] to the built [`Engine`].
    ///
    /// Returns a [`HostComponentDataHandle`] which can be passed to
    /// [`HostComponentsData`] to access or set associated
    /// [`HostComponent::Data`] for an instance.
    pub fn add_host_component<HC: HostComponent + Send + Sync + 'static>(
        &mut self,
        host_component: HC,
    ) -> Result<HostComponentDataHandle<HC>> {
        self.host_components_builder
            .add_host_component(&mut self.linker, host_component)
    }

    /// Sets the epoch tick internal for the built [`Engine`].
    ///
    /// This is used by [`Store::set_deadline`] to calculate the number of
    /// "ticks" for epoch interruption, and by the default epoch ticker thread.
    /// The default is [`DEFAULT_EPOCH_TICK_INTERVAL`].
    ///
    /// See [`EngineBuilder::epoch_ticker_thread`] and
    /// [`wasmtime::Config::epoch_interruption`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.epoch_interruption).
    pub fn epoch_tick_interval(&mut self, interval: Duration) {
        self.epoch_tick_interval = interval;
    }

    /// Configures whether the epoch ticker thread will be spawned when this
    /// [`Engine`] is built.
    ///
    /// Enabled by default; if disabled, the user must arrange to call
    /// `engine.as_ref().increment_epoch()` every `epoch_tick_interval` or
    /// interrupt-based features like `Store::set_deadline` will not work.
    pub fn epoch_ticker_thread(&mut self, enable: bool) {
        self.epoch_ticker_thread = enable;
    }

    fn maybe_spawn_epoch_ticker(&self) -> Option<Sender<()>> {
        if !self.epoch_ticker_thread {
            return None;
        }
        let engine = self.engine.clone();
        let interval = self.epoch_tick_interval;
        let (send, recv) = crossbeam_channel::bounded(0);
        std::thread::spawn(move || loop {
            match recv.recv_timeout(interval) {
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => (),
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                res => panic!("unexpected epoch_ticker_signal: {res:?}"),
            }
            engine.increment_epoch();
        });
        Some(send)
    }

    /// Builds an [`Engine`] from this builder with the given host state data.
    ///
    /// Note that this data will generally go entirely unused, but is needed
    /// by the implementation of [`Engine::instantiate_pre`]. If `T: Default`,
    /// it is probably preferable to use [`EngineBuilder::build`].
    pub fn build_with_data(self, instance_pre_data: T) -> Engine<T> {
        let epoch_ticker_signal = self.maybe_spawn_epoch_ticker();

        let host_components = self.host_components_builder.build();

        let instance_pre_store = Arc::new(Mutex::new(
            StoreBuilder::new(self.engine.clone(), Duration::ZERO, &host_components)
                .build_with_data(instance_pre_data)
                .expect("instance_pre_store build should not fail"),
        ));

        Engine {
            inner: self.engine,
            linker: self.linker,
            host_components,
            instance_pre_store,
            epoch_tick_interval: self.epoch_tick_interval,
            _epoch_ticker_signal: epoch_ticker_signal,
        }
    }
}

impl<T: Default + Send + Sync> EngineBuilder<T> {
    /// Builds an [`Engine`] from this builder.
    pub fn build(self) -> Engine<T> {
        self.build_with_data(T::default())
    }
}

/// An `Engine` is a global context for the initialization and execution of
/// Spin components.
pub struct Engine<T> {
    inner: wasmtime::Engine,
    linker: Linker<T>,
    host_components: HostComponents,
    instance_pre_store: Arc<Mutex<Store<T>>>,
    epoch_tick_interval: Duration,
    // Matching receiver closes on drop
    _epoch_ticker_signal: Option<Sender<()>>,
}

impl<T: Send + Sync> Engine<T> {
    /// Creates a new [`EngineBuilder`] with the given [`Config`].
    pub fn builder(config: &Config) -> Result<EngineBuilder<T>> {
        EngineBuilder::new(config)
    }

    /// Creates a new [`StoreBuilder`].
    pub fn store_builder(&self) -> StoreBuilder {
        StoreBuilder::new(
            self.inner.clone(),
            self.epoch_tick_interval,
            &self.host_components,
        )
    }

    /// Creates a new [`InstancePre`] for the given [`Module`].
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

/// A pre-initialized instance that is ready to be instantiated.
///
/// See [`wasmtime::InstancePre`] for more information.
pub struct InstancePre<T> {
    inner: wasmtime::InstancePre<Data<T>>,
}

impl<T: Send + Sync> InstancePre<T> {
    /// Instantiates this instance with the given [`Store`].
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
