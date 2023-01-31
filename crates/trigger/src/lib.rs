pub mod cli;
pub mod config;
pub mod loader;
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

use spin_app::{App, AppComponent, AppLoader, AppTrigger, Loader, OwnedApp};
use spin_config::{
    provider::{env::EnvProvider, vault::VaultProvider},
    Provider,
};
use spin_core::{Config, Engine, EngineBuilder, Instance, InstancePre, Store, StoreBuilder};

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
    hooks: Box<dyn TriggerHooks>,
    disable_default_host_components: bool,
    _phantom: PhantomData<Executor>,
}

impl<Executor: TriggerExecutor> TriggerExecutorBuilder<Executor> {
    /// Create a new TriggerExecutorBuilder with the given Application.
    pub fn new(loader: impl Loader + Send + Sync + 'static) -> Self {
        Self {
            loader: AppLoader::new(loader),
            config: Default::default(),
            hooks: Box::new(()),
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

    pub fn hooks(&mut self, hooks: impl TriggerHooks + 'static) -> &mut Self {
        self.hooks = Box::new(hooks);
        self
    }

    pub fn disable_default_host_components(&mut self) -> &mut Self {
        self.disable_default_host_components = true;
        self
    }

    pub async fn build(
        mut self,
        app_uri: String,
        builder_config: config::TriggerExecutorBuilderConfig,
    ) -> Result<Executor>
    where
        Executor::TriggerConfig: DeserializeOwned,
    {
        let engine = {
            let mut builder = Engine::builder(&self.config)?;

            if !self.disable_default_host_components {
                builder.add_host_component(outbound_redis::OutboundRedisComponent)?;
                builder.add_host_component(outbound_pg::OutboundPg::default())?;
                builder.add_host_component(outbound_mysql::OutboundMysql::default())?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    outbound_http::OutboundHttpComponent,
                )?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    spin_config::ConfigHostComponent::new(
                        self.get_config_providers(&app_uri, &builder_config),
                    ),
                )?;
            }

            Executor::configure_engine(&mut builder)?;
            builder.build()
        };

        let app = self.loader.load_owned_app(app_uri).await?;
        let app_name = app.borrowed().require_metadata("name")?;

        self.hooks.app_loaded(app.borrowed())?;

        // Run trigger executor
        Executor::new(TriggerAppEngine::new(engine, app_name, app, self.hooks).await?)
    }

    pub fn get_config_providers(
        &self,
        app_uri: &str,
        builder_config: &config::TriggerExecutorBuilderConfig,
    ) -> Vec<Box<dyn Provider>> {
        let mut providers = self.default_config_providers(app_uri);
        for config_provider in &builder_config.config_providers {
            let provider = match config_provider {
                config::ConfigProvider::Vault(vault_config) => VaultProvider::new(
                    &vault_config.url,
                    &vault_config.token,
                    &vault_config.mount,
                    vault_config.prefix.clone(),
                ),
            };
            providers.push(Box::new(provider));
        }
        providers
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
    // Trigger hooks
    hooks: Box<dyn TriggerHooks>,
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
        hooks: Box<dyn TriggerHooks>,
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
            let instance_pre = engine
                .instantiate_pre(&module)
                .map_err(decode_preinstantiation_error)
                .with_context(|| format!("Failed to instantiate component '{}'", component.id()))?;
            component_instance_pres.insert(component.id().to_string(), instance_pre);
        }

        Ok(Self {
            engine,
            app_name,
            app,
            hooks,
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
        let component = self.get_component(component_id)?;
        self.hooks
            .component_store_builder(component, &mut builder)?;
        Ok(builder)
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
        let component = self.get_component(component_id)?;

        // Build Store
        component.apply_store_config(&mut store_builder).await?;
        let mut store = store_builder.build()?;

        // Instantiate
        let instance = self
            .component_instance_pres
            .get(component_id)
            .expect("component_instance_pres missing valid component_id")
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

    fn get_component(&self, component_id: &str) -> Result<AppComponent> {
        self.app().get_component(component_id).with_context(|| {
            format!(
                "app {:?} has no component {:?}",
                self.app_name, component_id
            )
        })
    }
}

/// TriggerHooks allows a Spin environment to hook into a TriggerAppEngine's
/// configuration and execution processes.
pub trait TriggerHooks: Send + Sync {
    #![allow(unused_variables)]

    /// Called once, immediately after an App is loaded.
    fn app_loaded(&mut self, app: &App) -> Result<()> {
        Ok(())
    }

    /// Called while an AppComponent is being prepared for execution.
    /// Implementations may update the given StoreBuilder to change the
    /// environment of the instance to be executed.
    fn component_store_builder(
        &self,
        component: AppComponent,
        store_builder: &mut StoreBuilder,
    ) -> Result<()> {
        Ok(())
    }
}

impl TriggerHooks for () {}

pub fn parse_file_url(url: &str) -> Result<PathBuf> {
    url::Url::parse(url)
        .with_context(|| format!("Invalid URL: {url:?}"))?
        .to_file_path()
        .map_err(|_| anyhow!("Invalid file URL path: {url:?}"))
}

fn decode_preinstantiation_error(e: anyhow::Error) -> anyhow::Error {
    let err_text = e.to_string();

    if err_text.contains("unknown import") && err_text.contains("has not been defined") {
        // TODO: how to maintain this list?
        let sdk_imported_interfaces = &[
            "outbound-pg",
            "outbound-redis",
            "spin-config",
            "wasi_experimental_http",
            "wasi-outbound-http",
        ];

        if sdk_imported_interfaces
            .iter()
            .map(|s| format!("{s}::"))
            .any(|s| err_text.contains(&s))
        {
            return anyhow!(
                "{e}. Check that the component uses a SDK version that matches the Spin runtime."
            );
        }
    }

    e
}
