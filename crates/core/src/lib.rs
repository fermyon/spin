//! Spin core execution engine
//!
//! This crate provides low-level Wasm and WASI functionality required by Spin.
//! Most of this functionality consists of wrappers around [`wasmtime`] and
//! [`wasi_common`] that narrows the flexibility of `wasmtime` to the set of
//! features used by Spin (such as only supporting `wasmtime`'s async calling style).

#![deny(missing_docs)]

mod host_component;
mod io;
mod limits;
mod preview1;
mod store;
pub mod wasi_2023_10_18;
pub mod wasi_2023_11_10;

use std::sync::OnceLock;
use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use crossbeam_channel::Sender;
use http::Request;
use tracing::{field::Empty, instrument};
use wasmtime::{InstanceAllocationStrategy, PoolingAllocationConfig};
use wasmtime_wasi::ResourceTable;
use wasmtime_wasi_http::body::HyperOutgoingBody;
use wasmtime_wasi_http::types::{default_send_request, WasiHttpCtx, WasiHttpView};

use self::host_component::{HostComponents, HostComponentsBuilder};

pub use async_trait::async_trait;
pub use wasmtime::{
    self,
    component::{Component, Instance},
    Instance as ModuleInstance, Module, Trap,
};
pub use wasmtime_wasi::I32Exit;

pub use host_component::{
    AnyHostComponentDataHandle, HostComponent, HostComponentDataHandle, HostComponentsData,
};
pub use io::OutputBuffer;
pub use store::{Store, StoreBuilder, Wasi, WasiVersion};

/// The default [`EngineBuilder::epoch_tick_interval`].
pub const DEFAULT_EPOCH_TICK_INTERVAL: Duration = Duration::from_millis(10);

const MB: u64 = 1 << 20;
const GB: u64 = 1 << 30;
const WASM_PAGE_SIZE: u64 = 64 * 1024;

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

    /// Enable the Wasmtime compilation cache. If `path` is given it will override
    /// the system default path.
    ///
    /// For more information, see the [Wasmtime cache config documentation][docs].
    ///
    /// [docs]: https://docs.wasmtime.dev/cli-cache.html
    pub fn enable_cache(&mut self, config_path: &Option<PathBuf>) -> Result<()> {
        match config_path {
            Some(p) => self.inner.cache_config_load(p)?,
            None => self.inner.cache_config_load_default()?,
        };

        Ok(())
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
        inner.wasm_component_model(true);

        if use_pooling_allocator_by_default() {
            // By default enable the pooling instance allocator in Wasmtime. This
            // drastically reduces syscall/kernel overhead for wasm execution,
            // especially in async contexts where async stacks must be allocated.
            // The general goal here is that the default settings here rarely, if
            // ever, need to be modified. As a result there aren't fine-grained
            // knobs for each of these settings just yet and instead they're
            // generally set to defaults. Environment-variable-based fallbacks are
            // supported though as an escape valve for if this is a problem.
            let mut pooling_config = PoolingAllocationConfig::default();
            pooling_config
                .total_component_instances(env("SPIN_WASMTIME_INSTANCE_COUNT", 1_000))
                // This number accounts for internal data structures that Wasmtime allocates for each instance.
                // Instance allocation is proportional to the number of "things" in a wasm module like functions,
                // globals, memories, etc. Instance allocations are relatively small and are largely inconsequential
                // compared to other runtime state, but a number needs to be chosen here so a relatively large threshold
                // of 10MB is arbitrarily chosen. It should be unlikely that any reasonably-sized module hits this limit.
                .max_component_instance_size(
                    env("SPIN_WASMTIME_INSTANCE_SIZE", (10 * MB) as u32) as usize
                )
                .max_core_instances_per_component(env("SPIN_WASMTIME_CORE_INSTANCE_COUNT", 200))
                .max_tables_per_component(env("SPIN_WASMTIME_INSTANCE_TABLES", 20))
                .table_elements(env("SPIN_WASMTIME_INSTANCE_TABLE_ELEMENTS", 30_000))
                // The number of memories an instance can have effectively limits the number of inner components
                // a composed component can have (since each inner component has its own memory). We default to 32 for now, and
                // we'll see how often this limit gets reached.
                .max_memories_per_component(env("SPIN_WASMTIME_INSTANCE_MEMORIES", 32))
                .total_memories(env("SPIN_WASMTIME_TOTAL_MEMORIES", 1_000))
                .total_tables(env("SPIN_WASMTIME_TOTAL_TABLES", 2_000))
                // Nothing is lost from allowing the maximum size of memory for
                // all instance as it's still limited through other the normal
                // `StoreLimitsAsync` accounting method too.
                .memory_pages(4 * GB / WASM_PAGE_SIZE)
                // These numbers are completely arbitrary at something above 0.
                .linear_memory_keep_resident((2 * MB) as usize)
                .table_keep_resident((MB / 2) as usize);
            inner.allocation_strategy(InstanceAllocationStrategy::Pooling(pooling_config));
        }

        return Self { inner };

        fn env(name: &str, default: u32) -> u32 {
            match std::env::var(name) {
                Ok(val) => val
                    .parse()
                    .unwrap_or_else(|e| panic!("failed to parse env var `{name}={val}`: {e}")),
                Err(_) => default,
            }
        }
    }
}

/// The pooling allocator is tailor made for the `spin up` use case, so
/// try to use it when we can. The main cost of the pooling allocator, however,
/// is the virtual memory required to run it. Not all systems support the same
/// amount of virtual memory, for example some aarch64 and riscv64 configuration
/// only support 39 bits of virtual address space.
///
/// The pooling allocator, by default, will request 1000 linear memories each
/// sized at 6G per linear memory. This is 6T of virtual memory which ends up
/// being about 42 bits of the address space. This exceeds the 39 bit limit of
/// some systems, so there the pooling allocator will fail by default.
///
/// This function attempts to dynamically determine the hint for the pooling
/// allocator. This returns `true` if the pooling allocator should be used
/// by default, or `false` otherwise.
///
/// The method for testing this is to allocate a 0-sized 64-bit linear memory
/// with a maximum size that's N bits large where we force all memories to be
/// static. This should attempt to acquire N bits of the virtual address space.
/// If successful that should mean that the pooling allocator is OK to use, but
/// if it fails then the pooling allocator is not used and the normal mmap-based
/// implementation is used instead.
fn use_pooling_allocator_by_default() -> bool {
    static USE_POOLING: OnceLock<bool> = OnceLock::new();
    const BITS_TO_TEST: u32 = 42;

    *USE_POOLING.get_or_init(|| {
        // Enable manual control through env vars as an escape hatch
        match std::env::var("SPIN_WASMTIME_POOLING") {
            Ok(s) if s == "1" => return true,
            Ok(s) if s == "0" => return false,
            Ok(s) => panic!("SPIN_WASMTIME_POOLING={s} not supported, only 1/0 supported"),
            Err(_) => {}
        }

        // If the env var isn't set then perform the dynamic runtime probe
        let mut config = wasmtime::Config::new();
        config.wasm_memory64(true);
        config.static_memory_maximum_size(1 << BITS_TO_TEST);

        match wasmtime::Engine::new(&config) {
            Ok(engine) => {
                let mut store = wasmtime::Store::new(&engine, ());
                // NB: the maximum size is in wasm pages so take out the 16-bits
                // of wasm page size here from the maximum size.
                let ty = wasmtime::MemoryType::new64(0, Some(1 << (BITS_TO_TEST - 16)));
                wasmtime::Memory::new(&mut store, ty).is_ok()
            }
            Err(_) => {
                tracing::debug!(
                    "unable to create an engine to test the pooling \
                     allocator, disabling pooling allocation"
                );
                false
            }
        }
    })
}

/// Host state data associated with individual [Store]s and [Instance]s.
pub struct Data<T> {
    inner: T,
    wasi: Wasi,
    host_components_data: HostComponentsData,
    store_limits: limits::StoreLimitsAsync,
    table: ResourceTable,
}

impl<T> Data<T> {
    /// Get the amount of memory in bytes consumed by instances in the store
    pub fn memory_consumed(&self) -> u64 {
        self.store_limits.memory_consumed()
    }
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

impl<T: Send> wasmtime_wasi::WasiView for Data<T> {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        match &mut self.wasi {
            Wasi::Preview1(_) => panic!("using WASI Preview 1 functions with Preview 2 store"),
            Wasi::Preview2 { wasi_ctx, .. } => wasi_ctx,
        }
    }
}

impl<T: Send + OutboundWasiHttpHandler> WasiHttpView for Data<T> {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        match &mut self.wasi {
            Wasi::Preview1(_) => panic!("using WASI Preview 1 functions with Preview 2 store"),
            Wasi::Preview2 { wasi_http_ctx, .. } => wasi_http_ctx,
        }
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    #[instrument(
        name = "spin_core.send_request",
        skip_all,
        fields(
            otel.kind = "client",
            url.full = %request.uri(),
            http.request.method = %request.method(),
            otel.name = %request.method(),
            http.response.status_code = Empty,
            server.address = Empty,
            server.port = Empty,
        ),
    )]
    fn send_request(
        &mut self,
        mut request: Request<HyperOutgoingBody>,
        config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse>
    where
        Self: Sized,
    {
        spin_telemetry::inject_trace_context(&mut request);
        T::send_request(self, request, config)
    }
}

/// Handler for wasi-http based requests
pub trait OutboundWasiHttpHandler {
    /// Send the request
    fn send_request(
        data: &mut Data<Self>,
        request: Request<HyperOutgoingBody>,
        config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse>
    where
        Self: Sized;
}

impl OutboundWasiHttpHandler for () {
    fn send_request(
        _data: &mut Data<Self>,
        request: Request<HyperOutgoingBody>,
        config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse>
    where
        Self: Sized,
    {
        Ok(default_send_request(request, config))
    }
}

/// An alias for [`wasmtime::Linker`] specialized to [`Data`].
pub type ModuleLinker<T> = wasmtime::Linker<Data<T>>;

/// An alias for [`wasmtime::component::Linker`] specialized to [`Data`].
pub type Linker<T> = wasmtime::component::Linker<Data<T>>;

/// A builder interface for configuring a new [`Engine`].
///
/// A new [`EngineBuilder`] can be obtained with [`Engine::builder`].
pub struct EngineBuilder<T> {
    engine: wasmtime::Engine,
    linker: Linker<T>,
    module_linker: ModuleLinker<T>,
    host_components_builder: HostComponentsBuilder,
    epoch_tick_interval: Duration,
    epoch_ticker_thread: bool,
}

impl<T: Send + Sync + OutboundWasiHttpHandler> EngineBuilder<T> {
    fn new(config: &Config) -> Result<Self> {
        let engine = wasmtime::Engine::new(&config.inner)?;
        let linker: Linker<T> = Linker::new(&engine);
        let mut module_linker = ModuleLinker::new(&engine);

        wasi_common_preview1::tokio::add_to_linker(&mut module_linker, |data| {
            match &mut data.wasi {
                Wasi::Preview1(ctx) => ctx,
                Wasi::Preview2 { .. } => {
                    panic!("using WASI Preview 2 functions with Preview 1 store")
                }
            }
        })?;

        Ok(Self {
            engine,
            linker,
            module_linker,
            host_components_builder: HostComponents::builder(),
            epoch_tick_interval: DEFAULT_EPOCH_TICK_INTERVAL,
            epoch_ticker_thread: true,
        })
    }
}

impl<T: Send + Sync> EngineBuilder<T> {
    /// Adds definition(s) to the built [`Engine`].
    ///
    /// This method's signature is meant to be used with
    /// [`wasmtime::component::bindgen`]'s generated `add_to_linker` functions, e.g.:
    ///
    /// ```ignore
    /// use spin_core::my_interface;
    /// // ...
    /// let mut builder: EngineBuilder<my_interface::MyInterfaceData> = Engine::builder();
    /// builder.link_import(my_interface::add_to_linker)?;
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

    /// Builds an [`Engine`] from this builder.
    pub fn build(self) -> Engine<T> {
        let epoch_ticker_signal = self.maybe_spawn_epoch_ticker();

        let host_components = self.host_components_builder.build();

        Engine {
            inner: self.engine,
            linker: self.linker,
            module_linker: self.module_linker,
            host_components,
            epoch_tick_interval: self.epoch_tick_interval,
            _epoch_ticker_signal: epoch_ticker_signal,
        }
    }
}

/// An `Engine` is a global context for the initialization and execution of
/// Spin components.
pub struct Engine<T> {
    inner: wasmtime::Engine,
    linker: Linker<T>,
    module_linker: ModuleLinker<T>,
    host_components: HostComponents,
    epoch_tick_interval: Duration,
    // Matching receiver closes on drop
    _epoch_ticker_signal: Option<Sender<()>>,
}

impl<T: OutboundWasiHttpHandler + Send + Sync> Engine<T> {
    /// Creates a new [`EngineBuilder`] with the given [`Config`].
    pub fn builder(config: &Config) -> Result<EngineBuilder<T>> {
        EngineBuilder::new(config)
    }

    /// Creates a new [`StoreBuilder`].
    pub fn store_builder(&self, wasi_version: WasiVersion) -> StoreBuilder {
        StoreBuilder::new(
            self.inner.clone(),
            self.epoch_tick_interval,
            &self.host_components,
            wasi_version,
        )
    }

    /// Creates a new [`InstancePre`] for the given [`Component`].
    #[instrument(skip_all, level = "debug")]
    pub fn instantiate_pre(&self, component: &Component) -> Result<InstancePre<T>> {
        let inner = self.linker.instantiate_pre(component)?;
        Ok(InstancePre { inner })
    }

    /// Creates a new [`ModuleInstancePre`] for the given [`Module`].
    #[instrument(skip_all, level = "debug")]
    pub fn module_instantiate_pre(&self, module: &Module) -> Result<ModuleInstancePre<T>> {
        let inner = self.module_linker.instantiate_pre(module)?;
        Ok(ModuleInstancePre { inner })
    }

    /// Find the [`HostComponentDataHandle`] for a [`HostComponent`] if configured for this engine.
    /// Note: [`DynamicHostComponent`]s are implicitly wrapped in `Arc`s and need to be explicitly
    /// typed as such here, e.g. `find_host_component_handle::<Arc<MyDynamicHostComponent>>()`.
    pub fn find_host_component_handle<HC: HostComponent>(
        &self,
    ) -> Option<HostComponentDataHandle<HC>> {
        self.host_components.find_handle()
    }
}

impl<T> AsRef<wasmtime::Engine> for Engine<T> {
    fn as_ref(&self) -> &wasmtime::Engine {
        &self.inner
    }
}

/// A pre-initialized instance that is ready to be instantiated.
///
/// See [`wasmtime::component::InstancePre`] for more information.
pub struct InstancePre<T> {
    inner: wasmtime::component::InstancePre<Data<T>>,
}

impl<T: Send + Sync> InstancePre<T> {
    /// Instantiates this instance with the given [`Store`].
    #[instrument(skip_all, level = "debug")]
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

impl<T> AsRef<wasmtime::component::InstancePre<Data<T>>> for InstancePre<T> {
    fn as_ref(&self) -> &wasmtime::component::InstancePre<Data<T>> {
        &self.inner
    }
}

/// A pre-initialized module instance that is ready to be instantiated.
///
/// See [`wasmtime::InstancePre`] for more information.
pub struct ModuleInstancePre<T> {
    inner: wasmtime::InstancePre<Data<T>>,
}

impl<T: Send + Sync> ModuleInstancePre<T> {
    /// Instantiates this instance with the given [`Store`].
    #[instrument(skip_all, level = "debug")]
    pub async fn instantiate_async(&self, store: &mut Store<T>) -> Result<ModuleInstance> {
        self.inner.instantiate_async(store).await
    }
}

impl<T> Clone for ModuleInstancePre<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> AsRef<wasmtime::InstancePre<Data<T>>> for ModuleInstancePre<T> {
    fn as_ref(&self) -> &wasmtime::InstancePre<Data<T>> {
        &self.inner
    }
}
