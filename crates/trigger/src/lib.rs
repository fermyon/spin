pub mod cli;
mod loader;
pub mod locked;
mod stdio;

use std::{
    collections::HashMap,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
pub use async_trait::async_trait;
use serde::de::DeserializeOwned;

use spin_app::{App, AppLoader, AppTrigger, Loader, OwnedApp};
use spin_config::{provider::env::EnvProvider, Provider};
use spin_core::{Config, Engine, EngineBuilder, Instance, InstancePre, Store, StoreBuilder};

use stdio::{ComponentStdioWriter, FollowComponents};

const SPIN_HOME: &str = ".spin";
const SPIN_CONFIG_ENV_PREFIX: &str = "SPIN_APP";

#[async_trait]
pub trait TriggerExecutor: Sized {
    const TRIGGER_TYPE: &'static str;
    type RuntimeData: Default + Send + Sync + 'static;
    type TriggerConfig;
    type RunConfig;

    /// Create a new trigger executor.
    fn new(engine: TriggerAppEngine<Self>) -> Result<Self>;

    /// Run the trigger executor.
    async fn run(self, config: Self::RunConfig) -> Result<()>;

    /// Make changes to the ExecutionContext using the given Builder.
    fn configure_engine(_builder: &mut EngineBuilder<Self::RuntimeData>) -> Result<()> {
        Ok(())
    }
}

pub struct TriggerExecutorBuilder<Executor: TriggerExecutor> {
    loader: AppLoader,
    config: Config,
    log_dir: Option<PathBuf>,
    follow_components: FollowComponents,
    disable_default_host_components: bool,
    _phantom: PhantomData<Executor>,
}

impl<Executor: TriggerExecutor> TriggerExecutorBuilder<Executor> {
    /// Create a new TriggerExecutorBuilder with the given Application.
    pub fn new(loader: impl Loader + Send + Sync + 'static) -> Self {
        Self {
            loader: AppLoader::new(loader),
            config: Default::default(),
            log_dir: None,
            follow_components: Default::default(),
            disable_default_host_components: false,
            _phantom: PhantomData,
        }
    }

    /// !!!Warning!!! Using a custom Wasmtime Config is entirely unsupported;
    /// many configurations are likely to cause errors or unexpected behavior.
    #[doc(hidden)]
    pub fn wasmtime_config_mut(&mut self) -> &mut spin_core::wasmtime::Config {
        self.config.wasmtime_config()
    }

    pub fn log_dir(&mut self, log_dir: PathBuf) -> &mut Self {
        self.log_dir = Some(log_dir);
        self
    }

    pub fn follow_components(&mut self, follow_components: FollowComponents) -> &mut Self {
        self.follow_components = follow_components;
        self
    }

    pub fn disable_default_host_components(&mut self) -> &mut Self {
        self.disable_default_host_components = true;
        self
    }

    pub async fn build(mut self, app_uri: String) -> Result<Executor>
    where
        Executor::TriggerConfig: DeserializeOwned,
    {
        let engine = {
            let mut builder = Engine::builder(&self.config)?;

            if !self.disable_default_host_components {
                builder.add_host_component(outbound_redis::OutboundRedis::default())?;
                builder.add_host_component(outbound_pg::OutboundPg::default())?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    outbound_http::OutboundHttpComponent,
                )?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    spin_config::ConfigHostComponent::new(self.default_config_providers(&app_uri)),
                )?;
            }

            Executor::configure_engine(&mut builder)?;
            builder.build()
        };

        let app = self.loader.load_owned_app(app_uri).await?;
        let app_name = app.borrowed().require_metadata("name")?;

        let log_dir = {
            let sanitized_app = sanitize_filename::sanitize(&app_name);
            let parent_dir = match dirs::home_dir() {
                Some(home) => home.join(SPIN_HOME),
                None => PathBuf::new(), // "./"
            };
            parent_dir.join(sanitized_app).join("logs")
        };
        std::fs::create_dir_all(&log_dir)?;

        // Run trigger executor
        Executor::new(
            TriggerAppEngine::new(engine, app_name, app, log_dir, self.follow_components).await?,
        )
    }

    pub fn default_config_providers(&self, app_uri: &str) -> Vec<Box<dyn Provider>> {
        // EnvProvider
        // Look for a .env file in either the manifest parent directory for local apps
        // or the current directory for remote (e.g. bindle) apps.
        let dotenv_path = parse_file_url(app_uri)
            .as_deref()
            .ok()
            .unwrap_or_else(|| Path::new("."))
            .join(".env");
        vec![Box::new(EnvProvider::new(
            SPIN_CONFIG_ENV_PREFIX,
            Some(dotenv_path),
        ))]
    }
}

/// Execution context for a TriggerExecutor executing a particular App.
pub struct TriggerAppEngine<Executor: TriggerExecutor> {
    /// Engine to be used with this executor.
    pub engine: Engine<Executor::RuntimeData>,
    /// Name of the app for e.g. logging.
    pub app_name: String,
    // An owned wrapper of the App.
    app: OwnedApp,
    // Log directory
    log_dir: PathBuf,
    // Component stdio follow config
    follow_components: FollowComponents,
    // Trigger configs for this trigger type, with order matching `app.triggers_with_type(Executor::TRIGGER_TYPE)`
    trigger_configs: Vec<Executor::TriggerConfig>,
    // Map of {Component ID -> InstancePre} for each component.
    component_instance_pres: HashMap<String, InstancePre<Executor::RuntimeData>>,
}

impl<Executor: TriggerExecutor> TriggerAppEngine<Executor> {
    /// Returns a new TriggerAppEngine. May return an error if trigger config validation or
    /// component pre-instantiation fails.
    pub async fn new(
        engine: Engine<Executor::RuntimeData>,
        app_name: String,
        app: OwnedApp,
        log_dir: PathBuf,
        follow_components: FollowComponents,
    ) -> Result<Self>
    where
        <Executor as TriggerExecutor>::TriggerConfig: DeserializeOwned,
    {
        let trigger_configs = app
            .borrowed()
            .triggers_with_type(Executor::TRIGGER_TYPE)
            .map(|trigger| {
                trigger.typed_config().with_context(|| {
                    format!("invalid trigger configuration for {:?}", trigger.id())
                })
            })
            .collect::<Result<_>>()?;

        let mut component_instance_pres = HashMap::default();
        for component in app.borrowed().components() {
            let module = component.load_module(&engine).await?;
            let instance_pre = engine.instantiate_pre(&module)?;
            component_instance_pres.insert(component.id().to_string(), instance_pre);
        }

        Ok(Self {
            engine,
            app_name,
            app,
            log_dir,
            follow_components,
            trigger_configs,
            component_instance_pres,
        })
    }

    /// Returns a reference to the App.
    pub fn app(&self) -> &App {
        self.app.borrowed()
    }

    /// Returns AppTriggers and typed TriggerConfigs for this executor type.
    pub fn trigger_configs(&self) -> impl Iterator<Item = (AppTrigger, &Executor::TriggerConfig)> {
        self.app()
            .triggers_with_type(Executor::TRIGGER_TYPE)
            .zip(&self.trigger_configs)
    }

    /// Returns a new StoreBuilder for the given component ID.
    pub fn store_builder(&self, component_id: &str) -> Result<StoreBuilder> {
        let mut builder = self.engine.store_builder();

        // Set up stdio logging
        builder.stdout_pipe(self.component_stdio_writer(component_id, "stdout")?);
        builder.stderr_pipe(self.component_stdio_writer(component_id, "stderr")?);

        Ok(builder)
    }

    fn component_stdio_writer(
        &self,
        component_id: &str,
        log_suffix: &str,
    ) -> Result<ComponentStdioWriter> {
        let sanitized_component_id = sanitize_filename::sanitize(component_id);
        // e.g.
        let log_path = self
            .log_dir
            .join(format!("{sanitized_component_id}_{log_suffix}.txt"));
        let follow = self.follow_components.should_follow(component_id);
        ComponentStdioWriter::new(&log_path, follow)
            .with_context(|| format!("Failed to open log file {log_path:?}"))
    }

    /// Returns a new Store and Instance for the given component ID.
    pub async fn prepare_instance(
        &self,
        component_id: &str,
    ) -> Result<(Instance, Store<Executor::RuntimeData>)> {
        let store_builder = self.store_builder(component_id)?;
        self.prepare_instance_with_store(component_id, store_builder)
            .await
    }

    /// Returns a new Store and Instance for the given component ID and StoreBuilder.
    pub async fn prepare_instance_with_store(
        &self,
        component_id: &str,
        mut store_builder: StoreBuilder,
    ) -> Result<(Instance, Store<Executor::RuntimeData>)> {
        // Look up AppComponent
        let component = self.app().get_component(component_id).with_context(|| {
            format!(
                "app {:?} has no component {:?}",
                self.app_name, component_id
            )
        })?;

        // Build Store
        component.apply_store_config(&mut store_builder).await?;
        let mut store = store_builder.build()?;

        // Instantiate
        let instance = self.component_instance_pres[component_id]
            .instantiate_async(&mut store)
            .await
            .with_context(|| {
                format!(
                    "app {:?} component {:?} instantiation failed",
                    self.app_name, component_id
                )
            })?;

        Ok((instance, store))
    }
}

pub(crate) fn parse_file_url(url: &str) -> Result<PathBuf> {
    url::Url::parse(url)
        .with_context(|| format!("Invalid URL: {url:?}"))?
        .to_file_path()
        .map_err(|_| anyhow!("Invalid file URL path: {url:?}"))
}
