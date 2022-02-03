//! A Spin execution context for applications.

#![deny(missing_docs)]

use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use tracing::log;
use wasi_common::WasiCtx;
use wasmtime::{Engine, Instance, InstancePre, Linker, Module, Store};
use wasmtime_wasi::sync::{ambient_authority, Dir, WasiCtxBuilder};

/// Top-level configuration for a Spin execution context.
#[derive(Clone, Debug)]
pub struct Config {
    /// The Spin application configuration.
    pub spin: spin_config::Config,
    /// The Wasmtime engine configuration.
    pub wasmtime: wasmtime::Config,
}

impl Default for Config {
    fn default() -> Self {
        // In order for Wasmtime to run WebAssembly components, multi memory
        // and module linking must always be enabled.
        // See https://github.com/bytecodealliance/wit-bindgen/blob/main/crates/wasmlink
        let mut wasmtime = wasmtime::Config::default();
        wasmtime.wasm_multi_memory(true);
        wasmtime.wasm_module_linking(true);

        Self {
            wasmtime,
            ..Default::default()
        }
    }
}

/// Top-level runtime context data to be passed to a component.
#[derive(Default)]
pub struct RuntimeContext<T> {
    /// WASI context data.
    pub wasi: Option<WasiCtx>,
    /// Generic runtime data that can be configured by specialized engines.
    pub data: Option<T>,
}

/// Builder for the execution context.
#[derive(Default)]
pub struct ExecutionContextBuilder<T> {
    /// Configuration for the execution context builder.
    pub config: Config,
    /// Linker used to configure the execution context.
    pub linker: Linker<RuntimeContext<T>>,
    /// Store used to configure the execution context.
    pub store: Store<RuntimeContext<T>>,
    /// Wasmtime engine.
    pub engine: Engine,
}

impl<T: Default> ExecutionContextBuilder<T> {
    /// Create a new instance of the execution builder.
    #[tracing::instrument]
    pub fn new(entrypoint_path: String, config: Config) -> Result<ExecutionContextBuilder<T>> {
        let data = RuntimeContext::default();
        let engine = Engine::new(&config.wasmtime)?;
        let store = Store::new(&engine, data);
        let linker = Linker::new(&engine);

        Ok(Self {
            config,
            linker,
            store,
            engine,
        })
    }

    /// Configure the WASI linker imports for the current execution context.
    #[tracing::instrument(skip(self))]
    pub fn link_wasi<'a>(&'a mut self) -> Result<&'a Self> {
        wasmtime_wasi::add_to_linker(&mut self.linker, |ctx| ctx.wasi.as_mut().unwrap())?;
        Ok(self)
    }

    // TODO
    //
    // The current implementation of the outbound HTTP library makes it impossible to split
    // linking the implementation and passing runtime data, which means with this implementation,
    // we must either have a global list of allowed hosts per-application, or switch to the new
    // outbound HTTP implementation.
    // For now we skip adding outbound HTTP functionality by default.
}

/// A component of a Spin application.
#[derive(Clone)]
pub struct Component<T> {
    /// The configuration for a component.
    pub config: spin_config::Component,
    /// The pre-instance of the component.
    pub pre: Arc<InstancePre<RuntimeContext<T>>>,
}

impl<T> Component<T> {}

/// A generic execution context for WebAssembly components.
#[derive(Clone)]
pub struct ExecutionContext<T: Default> {
    /// Wasmtime engine.
    pub engine: Engine,
    /// Collection of pre-initialized (and already linked) components.
    pub components: HashMap<String, Component<T>>,
}

impl<T: Default> ExecutionContext<T> {
    /// Prepare a Wasm instance for the given component and actual data.
    #[tracing::instrument(skip(self, data))]
    pub fn prepare_component(
        &self,
        component: String,
        data: Option<T>,
    ) -> Result<(Store<RuntimeContext<T>>, Instance)> {
        let component = self
            .components
            .get(&component)
            .expect(&format!("cannot find component {}", component));
        let mut store = self.store(component, data)?;
        let instance = component.pre.instantiate(&mut store)?;

        Ok((store, instance))
    }

    /// Create a store for a given component given its configuration and runtime data.
    #[tracing::instrument(skip(self, component, data))]
    fn store(&self, component: &Component<T>, data: Option<T>) -> Result<Store<RuntimeContext<T>>> {
        log::info!("Creating store.");
        let (env, dirs) = Self::wasi_config(component)?;
        let mut ctx = RuntimeContext::default();
        let mut wasi_ctx = WasiCtxBuilder::new().inherit_stdio().envs(&env)?;

        for (guest, host) in dirs {
            wasi_ctx =
                wasi_ctx.preopened_dir(Dir::open_ambient_dir(host, ambient_authority())?, guest)?;
        }

        ctx.wasi = Some(wasi_ctx.build());
        ctx.data = data;

        let store = Store::new(&self.engine, ctx);
        Ok(store)
    }

    #[tracing::instrument(skip(component))]
    fn wasi_config(
        component: &Component<T>,
    ) -> Result<(Vec<(String, String)>, Vec<(String, String)>)> {
        todo!()
    }
}
