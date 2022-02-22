//! A Spin execution context for applications.

#![deny(missing_docs)]

/// Input / Output redirects.
pub mod io;

use anyhow::{bail, Result};
use io::IoStreamRedirects;
use spin_config::{CoreComponent, DirectoryMount, ModuleSource};
use std::{collections::HashMap, sync::Arc};
use tracing::{instrument, log};
use wasi_common::WasiCtx;
use wasmtime::{Engine, Instance, InstancePre, Linker, Module, Store};
use wasmtime_wasi::{ambient_authority, Dir, WasiCtxBuilder};

/// Runtime configuration
#[derive(Clone, Debug, Default)]
pub struct RuntimeConfig;

/// Builder-specific configuration.
#[derive(Clone, Debug)]
pub struct ExecutionContextConfiguration {
    /// Spin application configuration.
    pub app: spin_config::Configuration<CoreComponent>,
    /// Wasmtime engine configuration.
    pub wasmtime: wasmtime::Config,
}

impl ExecutionContextConfiguration {
    /// Create a new execution context configuration.
    pub fn new(app: spin_config::Configuration<CoreComponent>) -> Self {
        // In order for Wasmtime to run WebAssembly components, multi memory
        // and module linking must always be enabled.
        // See https://github.com/bytecodealliance/wit-bindgen/blob/main/crates/wasmlink.
        let mut wasmtime = wasmtime::Config::default();
        wasmtime.wasm_multi_memory(true);
        wasmtime.wasm_module_linking(true);
        wasmtime.async_support(true);

        log::trace!("Created execution context configuration.");

        Self { app, wasmtime }
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

/// An execution context builder.
pub struct Builder<T: Default> {
    /// Top level configuration for
    pub config: ExecutionContextConfiguration,
    /// Linker used to configure the execution context.
    pub linker: Linker<RuntimeContext<T>>,
    /// Store used to configure the execution context.
    pub store: Store<RuntimeContext<T>>,
    /// Wasmtime engine.
    pub engine: Engine,
}

impl<T: Default> Builder<T> {
    /// Create a new instance of the execution builder.
    pub fn new(config: ExecutionContextConfiguration) -> Result<Builder<T>> {
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
    pub fn link_wasi(&mut self) -> Result<&Self> {
        wasmtime_wasi::add_to_linker(&mut self.linker, |ctx| ctx.wasi.as_mut().unwrap())?;
        Ok(self)
    }

    // TODO
    //
    // The current implementation of the outbound HTTP library makes it impossible to split
    // linking the implementation and passing runtime data, which means with this implementation,
    // we must either have a global list of allowed hosts per-application, or switch to the new
    // outbound HTTP implementation.
    //
    // Importing the next version of the outbound HTTP library
    // from https://github.com/fermyon/wasi-experimental-toolkit/tree/main/crates/http-wasmtime
    // doesn't work as a git import, as it can't find the WIT file.
    //
    // For now, we skip adding outbound HTTP functionality by default.

    /// Build a new instance of the execution context.
    #[instrument(skip(self))]
    pub async fn build(&mut self) -> Result<ExecutionContext<T>> {
        let mut components = HashMap::new();
        for c in &self.config.app.components {
            let core = c.clone();

            // TODO
            let module = match c.source.clone() {
                ModuleSource::FileReference(p) => {
                    let module = Module::from_file(&self.engine, &p)?;
                    log::trace!("Created module from file {:?}", p);
                    module
                }
            };

            let pre = Arc::new(self.linker.instantiate_pre(&mut self.store, &module)?);
            log::debug!("Created pre-instance from module"); // TODO: show source?

            components.insert(c.id.clone(), Component { core, pre });
        }

        let config = self.config.clone();
        let engine = self.engine.clone();

        log::trace!("Execution context initialized.");

        Ok(ExecutionContext {
            config,
            engine,
            components,
        })
    }

    /// Build a new default instance of the execution context.
    pub async fn build_default(
        config: ExecutionContextConfiguration,
    ) -> Result<ExecutionContext<T>> {
        let mut builder = Self::new(config)?;
        builder.link_wasi()?;

        builder.build().await
    }
}

/// Component for the execution context.
#[derive(Clone)]
pub struct Component<T: Default> {
    /// Configuration for the component.
    pub core: CoreComponent,
    /// The pre-instance of the component
    pub pre: Arc<InstancePre<RuntimeContext<T>>>,
}

/// A generic execution context for WebAssembly components.
#[derive(Clone)]
pub struct ExecutionContext<T: Default> {
    /// Top-level runtime configuration.
    pub config: ExecutionContextConfiguration,
    /// Wasmtime engine.
    pub engine: Engine,
    /// Collection of pre-initialized (and already linked) components.
    pub components: HashMap<String, Component<T>>,
}

impl<T: Default> ExecutionContext<T> {
    /// Create a store for a given component given its configuration and runtime data.
    #[instrument(skip(self, data, io))]
    pub fn prepare_component(
        &self,
        component: &str,
        data: Option<T>,
        io: Option<IoStreamRedirects>,
        env: Option<HashMap<String, String>>,
        args: Option<Vec<String>>,
    ) -> Result<(Store<RuntimeContext<T>>, Instance)> {
        log::debug!("Preparing component {}", component);
        let component = match self.components.get(component) {
            Some(c) => c,
            None => bail!("Cannot find component {}", component),
        };

        let mut store = self.store(component, data, io, env, args)?;
        let instance = component.pre.instantiate(&mut store)?;

        Ok((store, instance))
    }

    /// Create a store for a given component given its configuration and runtime data.
    fn store(
        &self,
        component: &Component<T>,
        data: Option<T>,
        io: Option<IoStreamRedirects>,
        env: Option<HashMap<String, String>>,
        args: Option<Vec<String>>,
    ) -> Result<Store<RuntimeContext<T>>> {
        log::debug!("Creating store.");
        let (env, dirs) = Self::wasi_config(component, env)?;
        let mut ctx = RuntimeContext::default();
        let mut wasi_ctx = WasiCtxBuilder::new()
            .args(&args.unwrap_or_default())?
            .envs(&env)?;

        match io {
            Some(r) => {
                wasi_ctx = wasi_ctx
                    .stderr(Box::new(r.stderr.out))
                    // .inherit_stderr()
                    .stdout(Box::new(r.stdout.out))
                    .stdin(Box::new(r.stdin));
            }
            None => wasi_ctx = wasi_ctx.inherit_stdio(),
        };

        for dir in dirs {
            let guest = dir.guest;
            let host = dir.host;
            wasi_ctx =
                wasi_ctx.preopened_dir(Dir::open_ambient_dir(host, ambient_authority())?, guest)?;
        }

        ctx.wasi = Some(wasi_ctx.build());
        ctx.data = data;

        let store = Store::new(&self.engine, ctx);
        Ok(store)
    }

    #[allow(clippy::type_complexity)]
    fn wasi_config(
        component: &Component<T>,
        env: Option<HashMap<String, String>>,
    ) -> Result<(Vec<(String, String)>, Vec<DirectoryMount>)> {
        let mut res = vec![];

        for (k, v) in &component.core.wasm.environment {
            res.push((k.clone(), v.clone()));
        }

        // Custom environment variables currently take precedence over component-defined
        // environment variables. This might change in the future.
        if let Some(envs) = env {
            for (k, v) in envs {
                res.push((k.clone(), v.clone()));
            }
        };

        let dirs = component.core.wasm.mounts.clone();

        Ok((res, dirs))
    }
}
