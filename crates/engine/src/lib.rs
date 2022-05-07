//! A Spin execution context for applications.

#![deny(missing_docs)]

/// Host components.
pub mod host_component;
/// Input / Output redirects.
pub mod io;

use anyhow::{bail, Context, Result};
use host_component::{HostComponent, HostComponents, HostComponentsState};
use io::IoStreamRedirects;
use spin_config::{host_component::ComponentConfig, Resolver};
use spin_manifest::{Application, CoreComponent, DirectoryMount, ModuleSource};
use std::{collections::HashMap, io::Write, path::PathBuf, sync::Arc};
use tokio::{
    task::JoinHandle,
    time::{sleep, Duration},
};
use tracing::{instrument, log};
use wasi_common::WasiCtx;
use wasmtime::{Engine, Instance, InstancePre, Linker, Module, Store};
use wasmtime_wasi::{ambient_authority, Dir, WasiCtxBuilder};

const SPIN_HOME: &str = ".spin";

/// Builder-specific configuration.
#[derive(Clone, Debug, Default)]
pub struct ExecutionContextConfiguration {
    /// Component configuration.
    pub components: Vec<CoreComponent>,
    /// Label for logging, etc.
    pub label: String,
    /// Log directory on host.
    pub log_dir: Option<PathBuf>,
    /// Application configuration resolver.
    pub config_resolver: Option<Arc<Resolver>>,
}

impl From<Application> for ExecutionContextConfiguration {
    fn from(app: Application) -> Self {
        Self {
            components: app.components,
            label: app.info.name,
            config_resolver: app.config_resolver,
            ..Default::default()
        }
    }
}

/// Top-level runtime context data to be passed to a component.
#[derive(Default)]
pub struct RuntimeContext<T> {
    /// WASI context data.
    pub wasi: Option<WasiCtx>,
    /// Component configuration.
    pub component_config: Option<spin_config::host_component::ComponentConfig>,
    /// Host components state.
    pub host_components_state: HostComponentsState,
    /// Generic runtime data that can be configured by specialized engines.
    pub data: Option<T>,
}

/// An execution context builder.
pub struct Builder<T: Default> {
    config: ExecutionContextConfiguration,
    linker: Linker<RuntimeContext<T>>,
    store: Store<RuntimeContext<T>>,
    engine: Engine,
    host_components: HostComponents,
}

impl<T: Default + 'static> Builder<T> {
    /// Creates a new instance of the execution builder.
    pub fn new(config: ExecutionContextConfiguration) -> Result<Builder<T>> {
        Self::with_wasmtime_config(config, Default::default())
    }

    /// Creates a new instance of the execution builder with the given wasmtime::Config.
    pub fn with_wasmtime_config(
        config: ExecutionContextConfiguration,
        mut wasmtime: wasmtime::Config,
    ) -> Result<Builder<T>> {
        // In order for Wasmtime to run WebAssembly components, multi memory
        // and module linking must always be enabled.
        // See https://github.com/bytecodealliance/wit-bindgen/blob/main/crates/wasmlink.
        wasmtime.wasm_multi_memory(true);
        wasmtime.wasm_module_linking(true);

        let data = RuntimeContext::default();
        let engine = Engine::new(&wasmtime)?;
        let store = Store::new(&engine, data);
        let linker = Linker::new(&engine);
        let host_components = Default::default();

        Ok(Self {
            config,
            linker,
            store,
            engine,
            host_components,
        })
    }

    /// Configures the WASI linker imports for the current execution context.
    pub fn link_wasi(&mut self) -> Result<&mut Self> {
        wasmtime_wasi::add_to_linker(&mut self.linker, |ctx| ctx.wasi.as_mut().unwrap())?;
        Ok(self)
    }

    /// Configures the application configuration interface.
    pub fn link_config(&mut self) -> Result<&mut Self> {
        spin_config::host_component::add_to_linker(&mut self.linker, |ctx| {
            ctx.component_config.as_mut().unwrap()
        })?;
        Ok(self)
    }

    /// Adds a HostComponent to the execution context.
    pub fn add_host_component(
        &mut self,
        host_component: impl HostComponent + 'static,
    ) -> Result<&mut Self> {
        self.host_components
            .insert(&mut self.linker, host_component)?;
        Ok(self)
    }

    /// Builds a new instance of the execution context.
    #[instrument(skip(self))]
    pub async fn build(mut self) -> Result<ExecutionContext<T>> {
        let _sloth_warning = warn_if_slothful();
        let mut components = HashMap::new();
        for c in &self.config.components {
            let core = c.clone();
            let module = match c.source.clone() {
                ModuleSource::FileReference(p) => {
                    let module = Module::from_file(&self.engine, &p).with_context(|| {
                        format!(
                            "Cannot create module for component {} from file {}",
                            &c.id,
                            &p.display()
                        )
                    })?;
                    log::trace!("Created module for component {} from file {:?}", &c.id, &p);
                    module
                }
                ModuleSource::Buffer(bytes, info) => {
                    let module = Module::from_binary(&self.engine, &bytes).with_context(|| {
                        format!("Cannot create module for component {} from {}", &c.id, info)
                    })?;
                    log::trace!(
                        "Created module for component {} from {} with size {}",
                        &c.id,
                        info,
                        bytes.len()
                    );
                    module
                }
            };

            let pre = Arc::new(self.linker.instantiate_pre(&mut self.store, &module)?);
            log::trace!("Created pre-instance from module for component {}.", &c.id);

            components.insert(c.id.clone(), Component { core, pre });
        }

        log::trace!("Execution context initialized.");

        Ok(ExecutionContext {
            config: self.config,
            engine: self.engine,
            components,
            host_components: Arc::new(self.host_components),
        })
    }

    /// Configures default host interface implementations.
    pub fn link_defaults(&mut self) -> Result<&mut Self> {
        self.link_wasi()?.link_config()
    }

    /// Builds a new default instance of the execution context.
    pub async fn build_default(
        config: ExecutionContextConfiguration,
    ) -> Result<ExecutionContext<T>> {
        let mut builder = Self::new(config)?;
        builder.link_defaults()?;
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

    host_components: Arc<HostComponents>,
}

impl<T: Default> ExecutionContext<T> {
    /// Creates a store for a given component given its configuration and runtime data.
    #[instrument(skip(self, data, io))]
    pub fn prepare_component(
        &self,
        component: &str,
        data: Option<T>,
        io: Option<IoStreamRedirects>,
        env: Option<HashMap<String, String>>,
        args: Option<Vec<String>>,
    ) -> Result<(Store<RuntimeContext<T>>, Instance)> {
        log::trace!("Preparing component {}", component);
        let component = match self.components.get(component) {
            Some(c) => c,
            None => bail!("Cannot find component {}", component),
        };

        let mut store = self.store(component, data, io, env, args)?;
        let instance = component.pre.instantiate(&mut store)?;

        Ok((store, instance))
    }

    /// Save logs for a given component in the log directory on the host
    pub fn save_output_to_logs(
        &self,
        io_redirects: IoStreamRedirects,
        component: &str,
        save_stdout: bool,
        save_stderr: bool,
    ) -> Result<()> {
        let sanitized_label = sanitize(&self.config.label);
        let sanitized_component_name = sanitize(&component);

        let log_dir = match &self.config.log_dir {
            Some(l) => l.clone(),
            None => match dirs::home_dir() {
                Some(h) => h.join(SPIN_HOME).join(&sanitized_label).join("logs"),
                None => PathBuf::from(&sanitized_label).join("logs"),
            },
        };

        let stdout_filename = log_dir.join(sanitize(format!(
            "{}_{}.txt",
            sanitized_component_name, "stdout",
        )));

        let stderr_filename = log_dir.join(sanitize(format!(
            "{}_{}.txt",
            sanitized_component_name, "stderr"
        )));

        std::fs::create_dir_all(&log_dir)?;

        log::trace!("Saving logs to {:?} {:?}", stdout_filename, stderr_filename);

        if save_stdout {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(stdout_filename)?;
            let contents = io_redirects.stdout.lock.read().unwrap();
            file.write_all(&contents)?;
        }

        if save_stderr {
            let contents = io_redirects.stderr.lock.read().unwrap();
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(stderr_filename)?;
            file.write_all(&contents)?;
        }

        Ok(())
    }
    /// Creates a store for a given component given its configuration and runtime data.
    fn store(
        &self,
        component: &Component<T>,
        data: Option<T>,
        io: Option<IoStreamRedirects>,
        env: Option<HashMap<String, String>>,
        args: Option<Vec<String>>,
    ) -> Result<Store<RuntimeContext<T>>> {
        log::trace!("Creating store.");
        let (env, dirs) = Self::wasi_config(component, env)?;
        let mut ctx = RuntimeContext::default();
        let mut wasi_ctx = WasiCtxBuilder::new()
            .args(&args.unwrap_or_default())?
            .envs(&env)?;
        match io {
            Some(r) => {
                wasi_ctx = wasi_ctx
                    .stderr(Box::new(r.stderr.out))
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

        if let Some(resolver) = &self.config.config_resolver {
            ctx.component_config =
                Some(ComponentConfig::new(&component.core.id, resolver.clone())?);
        }

        ctx.host_components_state = self.host_components.build_state(&component.core)?;

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

fn sanitize(name: impl AsRef<str>) -> String {
    // options block copied from sanitize_filename project readme
    let options = sanitize_filename::Options {
        // true by default, truncates to 255 bytes
        truncate: true,
        // default value depends on the OS, removes reserved names like `con` from start of strings on Windows
        windows: true,
        // str to replace sanitized chars/strings
        replacement: "",
    };

    // filename logic defined in the project works for directory names as well
    // refer to: https://github.com/kardeiz/sanitize-filename/blob/f5158746946ed81015c3a33078dedf164686da19/src/lib.rs#L76-L165
    sanitize_filename::sanitize_with_options(name, options)
}

const SLOTH_WARNING_DELAY_MILLIS: u64 = 1250;

struct SlothWarning<T> {
    warning: JoinHandle<T>,
}

impl<T> Drop for SlothWarning<T> {
    fn drop(&mut self) {
        self.warning.abort()
    }
}

fn warn_if_slothful() -> SlothWarning<()> {
    let warning = tokio::spawn(warn_slow());
    SlothWarning { warning }
}

#[cfg(debug_assertions)]
async fn warn_slow() {
    sleep(Duration::from_millis(SLOTH_WARNING_DELAY_MILLIS)).await;
    println!("This is a debug build - preparing Wasm modules might take a few seconds");
    println!("If you're experiencing long startup times please switch to the release build");
    println!();
}

#[cfg(not(debug_assertions))]
async fn warn_slow() {
    sleep(Duration::from_millis(SLOTH_WARNING_DELAY_MILLIS)).await;
    println!("Preparing Wasm modules is taking a few seconds...");
    println!();
}
