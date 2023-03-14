pub mod cli;
mod key_value;
pub mod loader;
pub mod locked;
pub mod runtime_config;
mod stdio;

use std::{collections::HashMap, marker::PhantomData, path::PathBuf};

use anyhow::{anyhow, Context, Result};
pub use async_trait::async_trait;
use runtime_config::RuntimeConfig;
use serde::de::DeserializeOwned;

use spin_app::{App, AppComponent, AppLoader, AppTrigger, Loader, OwnedApp};
use spin_core::{Config, Engine, EngineBuilder, Instance, InstancePre, Store, StoreBuilder};

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
    hooks: Vec<Box<dyn TriggerHooks>>,
    disable_default_host_components: bool,
    _phantom: PhantomData<Executor>,
}

impl<Executor: TriggerExecutor> TriggerExecutorBuilder<Executor> {
    /// Create a new TriggerExecutorBuilder with the given Application.
    pub fn new(loader: impl Loader + Send + Sync + 'static) -> Self {
        Self {
            loader: AppLoader::new(loader),
            config: Default::default(),
            hooks: Default::default(),
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
        self.hooks.push(Box::new(hooks));
        self
    }

    pub fn disable_default_host_components(&mut self) -> &mut Self {
        self.disable_default_host_components = true;
        self
    }

    pub async fn build(
        mut self,
        app_uri: String,
        runtime_config: runtime_config::RuntimeConfig,
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
                    crate::key_value::build_key_value_component(&runtime_config)?,
                )?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    outbound_http::OutboundHttpComponent,
                )?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    spin_config::ConfigHostComponent::new(runtime_config.config_providers()),
                )?;
            }

            Executor::configure_engine(&mut builder)?;
            builder.build()
        };

        let app = self.loader.load_owned_app(app_uri).await?;

        let app_name = app.borrowed().require_metadata("name")?;

        self.hooks
            .iter_mut()
            .try_for_each(|h| h.app_loaded(app.borrowed(), &runtime_config))?;

        // Run trigger executor
        Executor::new(TriggerAppEngine::new(engine, app_name, app, self.hooks).await?)
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
    hooks: Vec<Box<dyn TriggerHooks>>,
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
        hooks: Vec<Box<dyn TriggerHooks>>,
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
            .iter()
            .try_for_each(|h| h.component_store_builder(&component, &mut builder))?;
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
    fn app_loaded(&mut self, app: &App, runtime_config: &RuntimeConfig) -> Result<()> {
        Ok(())
    }

    /// Called while an AppComponent is being prepared for execution.
    /// Implementations may update the given StoreBuilder to change the
    /// environment of the instance to be executed.
    fn component_store_builder(
        &self,
        component: &AppComponent,
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
