//! A Spin execution context for applications.

#![deny(missing_docs)]

use std::{collections::HashMap, sync::Arc};

use anyhow::{bail, Context, Result};
use spin_config::{ApplicationOrigin, CoreComponent, ModuleSource};
use tracing::{instrument, log};
use wasi_common::WasiCtx;
use wasmtime::{Engine, Instance, InstancePre, Linker, Module, Store};
use wasmtime_wasi::{ambient_authority, Dir, WasiCtxBuilder};

mod assets;
use assets::{prepare_local_assets, DirectoryMount};

/// Runtime configuration
#[derive(Clone, Debug, Default)]
pub struct RuntimeConfig;

const EMPTY: Vec<String> = vec![];

/// Builder-specific configuration.
#[derive(Clone, Debug)]
pub struct ExecutionContextConfiguration {
    /// Spin application configuration.
    /// TODO
    ///
    /// This should be Config<StartupComponentConfig> or something like that.
    pub app: spin_config::Configuration<CoreComponent>,
    /// Wasmtime engine configuration.
    pub wasmtime: wasmtime::Config,
}

impl ExecutionContextConfiguration {
    /// Create a new execution context configuration.
    #[instrument]
    pub fn new(app: spin_config::Configuration<CoreComponent>) -> Self {
        // In order for Wasmtime to run WebAssembly components, multi memory
        // and module linking must always be enabled.
        // See https://github.com/bytecodealliance/wit-bindgen/blob/main/crates/wasmlink.
        let mut wasmtime = wasmtime::Config::default();
        wasmtime.wasm_multi_memory(true);
        wasmtime.wasm_module_linking(true);

        log::info!("Created execution context configuration.");

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
    #[instrument]
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
    #[instrument(skip(self))]
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
    //
    // Importing the next version of the outbound HTTP library
    // from https://github.com/fermyon/wasi-experimental-toolkit/tree/main/crates/http-wasmtime
    // doesn't work as a git import, as it can't find the WIT file.
    //
    // For now, we skip adding outbound HTTP functionality by default.

    /// Build a new instance of the execution context.
    #[instrument(skip(self))]
    pub async fn build(&mut self) -> Result<ExecutionContext<T>> {
        let working_directory = tempfile::tempdir()?;
        log::debug!(
            "Created temporary directory '{}'",
            working_directory.path().display()
        );

        let mut components = HashMap::new();
        for c in &self.config.app.components {
            let config = c.clone();

            // TODO
            let p = match c.source.clone() {
                ModuleSource::FileReference(p) => p,
                ModuleSource::Bindle(_) => panic!(),
                ModuleSource::Linked(_) => panic!(),
            };

            let module = Module::from_file(&self.engine, &p)?;
            log::info!("Created module from file {:?}", p);
            let pre = Arc::new(self.linker.instantiate_pre(&mut self.store, &module)?);
            log::info!("Created pre-instance from module {:?}", p);

            let asset_directories = self.prepare_assets(c, working_directory.path()).await?;

            components.insert(
                c.id.clone(),
                Component {
                    core: config,
                    pre,
                    prepared_directories: asset_directories,
                },
            );
        }

        let config = self.config.clone();
        let engine = self.engine.clone();

        log::info!("Execution context initialized.");

        Ok(ExecutionContext {
            config,
            engine,
            components,
            working_directory: Arc::new(working_directory),
        })
    }

    async fn prepare_assets(
        &self,
        component: &CoreComponent,
        working_directory: impl AsRef<std::path::Path>,
    ) -> Result<Vec<DirectoryMount>> {
        match &self.config.app.info.origin {
            ApplicationOrigin::File(config_file) => {
                let files = component.wasm.files.as_ref();
                let source_directory = config_file
                    .parent()
                    .expect("The root directory cannot be the config file");
                let mount = prepare_local_assets(
                    files.unwrap_or(&EMPTY),
                    source_directory,
                    &working_directory,
                    &component.id,
                )
                .await
                .with_context(|| {
                    format!("Error copying assets for component '{}'", component.id)
                })?;
                Ok(vec![mount])
            }
        }
    }

    /// Build a new default instance of the execution context.
    #[instrument]
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
    /// TODO
    ///
    /// This should not be LocalComponentConfig.
    pub core: CoreComponent,
    /// The pre-instance of the component
    pub pre: Arc<InstancePre<RuntimeContext<T>>>,
    prepared_directories: Vec<DirectoryMount>,
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
    /// A directory for resources that need to last as long as the exeuction context
    /// but can then be deleted.
    working_directory: Arc<tempfile::TempDir>,
}

impl<T: Default> ExecutionContext<T> {
    /// Create a store for a given component given its configuration and runtime data.
    #[instrument(skip(self, data))]
    pub fn prepare_component(
        &self,
        component: &str,
        data: Option<T>,
    ) -> Result<(Store<RuntimeContext<T>>, Instance)> {
        log::info!("Preparing component {}", component);
        let component = match self.components.get(component) {
            Some(c) => c,
            None => bail!("Cannot find component {}", component),
        };

        let mut store = self.store(component, data)?;
        let instance = component.pre.instantiate(&mut store)?;

        Ok((store, instance))
    }

    /// Create a store for a given component given its configuration and runtime data.
    #[instrument(skip(self, component, data))]
    fn store(&self, component: &Component<T>, data: Option<T>) -> Result<Store<RuntimeContext<T>>> {
        log::info!("Creating store.");
        let (env, dirs) = Self::wasi_config(component)?;
        let mut ctx = RuntimeContext::default();
        let mut wasi_ctx = WasiCtxBuilder::new().inherit_stdio().envs(&env)?;

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

    #[instrument(skip(component))]
    #[allow(clippy::type_complexity)]
    fn wasi_config(
        component: &Component<T>,
    ) -> Result<(Vec<(String, String)>, Vec<DirectoryMount>)> {
        let mut env = vec![];

        if let Some(e) = &component.core.wasm.environment {
            for (k, v) in e {
                env.push((k.clone(), v.clone()));
            }
        };

        let dirs = component.prepared_directories.clone();

        Ok((env, dirs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use spin_config::{ApplicationOrigin, Configuration, RawConfiguration};

    const CFG_TEST: &str = r#"
    name        = "spin-hello-world"
    version     = "1.0.0"
    description = "A simple application that returns hello and goodbye."
    authors     = [ "Radu Matei <radu@fermyon.com>" ]
    trigger     = "http"

    [[component]]
        source = "target/wasm32-wasi/release/hello.wasm"
        id     = "hello"
    [component.trigger]
        route = "/hello"
    "#;

    fn fake_file_origin() -> ApplicationOrigin {
        let dir = env!("CARGO_MANIFEST_DIR");
        let fake_path = std::path::PathBuf::from(dir).join("fake_spin.toml");
        ApplicationOrigin::File(fake_path)
    }

    #[test]
    fn test_simple_config() -> Result<()> {
        let raw_app: RawConfiguration<CoreComponent> = toml::from_str(CFG_TEST)?;
        let app = Configuration::from_raw(raw_app, fake_file_origin());
        let config = ExecutionContextConfiguration::new(app);

        assert_eq!(config.app.info.name, "spin-hello-world".to_string());
        Ok(())
    }
}
