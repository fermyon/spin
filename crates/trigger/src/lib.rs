pub mod cli;
pub mod loader;
pub mod network;
mod runtime_config;
mod stdio;

use std::{collections::HashMap, marker::PhantomData};

use anyhow::{Context, Result};
pub use async_trait::async_trait;
use runtime_config::llm::LLmOptions;
use serde::de::DeserializeOwned;

use spin_app::{App, AppComponent, AppLoader, AppTrigger, Loader, OwnedApp, APP_NAME_KEY};
use spin_core::{
    Config, Engine, EngineBuilder, Instance, InstancePre, ModuleInstance, ModuleInstancePre,
    OutboundWasiHttpHandler, Store, StoreBuilder, WasiVersion,
};

pub use crate::runtime_config::RuntimeConfig;

pub enum EitherInstancePre<T> {
    Component(InstancePre<T>),
    Module(ModuleInstancePre<T>),
}

pub enum EitherInstance {
    Component(Instance),
    Module(ModuleInstance),
}

#[async_trait]
pub trait TriggerExecutor: Sized + Send + Sync {
    const TRIGGER_TYPE: &'static str;
    type RuntimeData: OutboundWasiHttpHandler + Default + Send + Sync + 'static;
    type TriggerConfig;
    type RunConfig;

    /// Create a new trigger executor.
    async fn new(engine: TriggerAppEngine<Self>) -> Result<Self>;

    /// Run the trigger executor.
    async fn run(self, config: Self::RunConfig) -> Result<()>;

    /// Make changes to the ExecutionContext using the given Builder.
    fn configure_engine(_builder: &mut EngineBuilder<Self::RuntimeData>) -> Result<()> {
        Ok(())
    }

    async fn instantiate_pre(
        engine: &Engine<Self::RuntimeData>,
        component: &AppComponent,
        _config: &Self::TriggerConfig,
    ) -> Result<EitherInstancePre<Self::RuntimeData>> {
        let comp = component.load_component(engine).await?;
        Ok(EitherInstancePre::Component(
            engine
                .instantiate_pre(&comp)
                .with_context(|| format!("Failed to instantiate component '{}'", component.id()))?,
        ))
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
    pub fn config_mut(&mut self) -> &mut spin_core::Config {
        &mut self.config
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
        init_data: HostComponentInitData,
    ) -> Result<Executor>
    where
        Executor::TriggerConfig: DeserializeOwned,
    {
        let engine = {
            let mut builder = Engine::builder(&self.config)?;

            if !self.disable_default_host_components {
                // Wasmtime 17: WASI@0.2.0
                builder.link_import(|l, _| {
                    wasmtime_wasi::preview2::command::add_to_linker(l)?;
                    wasmtime_wasi_http::proxy::add_only_http_to_linker(l)
                })?;

                // Wasmtime 15: WASI@0.2.0-rc-2023-11-10
                builder.link_import(|l, _| spin_core::wasi_2023_11_10::add_to_linker(l))?;

                // Wasmtime 14: WASI@0.2.0-rc-2023-10-18
                builder.link_import(|l, _| spin_core::wasi_2023_10_18::add_to_linker(l))?;

                self.loader.add_dynamic_host_component(
                    &mut builder,
                    outbound_redis::OutboundRedisComponent,
                )?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    outbound_mqtt::OutboundMqttComponent,
                )?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    outbound_mysql::OutboundMysql::default(),
                )?;
                self.loader
                    .add_dynamic_host_component(&mut builder, outbound_pg::OutboundPg::default())?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    runtime_config::llm::build_component(&runtime_config, init_data.llm.use_gpu)
                        .await,
                )?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    runtime_config::key_value::build_key_value_component(
                        &runtime_config,
                        &init_data.kv,
                    )
                    .await?,
                )?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    runtime_config::sqlite::build_component(&runtime_config, &init_data.sqlite)
                        .await?,
                )?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    outbound_http::OutboundHttpComponent,
                )?;
                self.loader.add_dynamic_host_component(
                    &mut builder,
                    spin_variables::VariablesHostComponent::new(
                        runtime_config.variables_providers(),
                    ),
                )?;
            }

            Executor::configure_engine(&mut builder)?;
            builder.build()
        };

        let app = self.loader.load_owned_app(app_uri).await?;

        let app_name = app.borrowed().require_metadata(APP_NAME_KEY)?;

        self.hooks
            .iter_mut()
            .try_for_each(|h| h.app_loaded(app.borrowed(), &runtime_config))?;

        // Run trigger executor
        Executor::new(TriggerAppEngine::new(engine, app_name, app, self.hooks).await?).await
    }
}

/// Initialization data for host components.
#[derive(Default)] // TODO: the implementation of Default is only for tests - would like to get rid of
pub struct HostComponentInitData {
    kv: Vec<(String, String)>,
    sqlite: Vec<String>,
    llm: LLmOptions,
}

impl HostComponentInitData {
    /// Create an instance of `HostComponentInitData`.  `key_value_init_values`
    /// will be added to the default key-value store; `sqlite_init_statements`
    /// will be run against the default SQLite database.
    pub fn new(
        key_value_init_values: impl Into<Vec<(String, String)>>,
        sqlite_init_statements: impl Into<Vec<String>>,
        llm: LLmOptions,
    ) -> Self {
        Self {
            kv: key_value_init_values.into(),
            sqlite: sqlite_init_statements.into(),
            llm,
        }
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
    component_instance_pres: HashMap<String, EitherInstancePre<Executor::RuntimeData>>,
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
                Ok((
                    trigger.component()?.id().to_owned(),
                    trigger.typed_config().with_context(|| {
                        format!("invalid trigger configuration for {:?}", trigger.id())
                    })?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        let mut component_instance_pres = HashMap::default();
        for component in app.borrowed().components() {
            let id = component.id();
            // There is an issue here for triggers that consider the trigger config during
            // preinstantiation. We defer this for now because the only case is the HTTP
            // `executor` field and that should not differ from trigger to trigger.
            let trigger_config = trigger_configs
                .iter()
                .find(|(c, _)| c == id)
                .map(|(_, cfg)| cfg);
            if let Some(config) = trigger_config {
                component_instance_pres.insert(
                    id.to_owned(),
                    Executor::instantiate_pre(&engine, &component, config)
                        .await
                        .with_context(|| format!("Failed to instantiate component '{id}'"))?,
                );
            } else {
                tracing::warn!(
                    "component '{id}' is not used by any triggers in app '{app_name}'",
                    id = id,
                    app_name = app_name
                )
            }
        }

        Ok(Self {
            engine,
            app_name,
            app,
            hooks,
            trigger_configs: trigger_configs.into_iter().map(|(_, v)| v).collect(),
            component_instance_pres,
        })
    }

    /// Returns a reference to the App.
    pub fn app(&self) -> &App {
        self.app.borrowed()
    }

    pub fn trigger_metadata<T: DeserializeOwned + Default>(&self) -> spin_app::Result<Option<T>> {
        self.app().get_trigger_metadata(Executor::TRIGGER_TYPE)
    }

    /// Returns AppTriggers and typed TriggerConfigs for this executor type.
    pub fn trigger_configs(&self) -> impl Iterator<Item = (AppTrigger, &Executor::TriggerConfig)> {
        self.app()
            .triggers_with_type(Executor::TRIGGER_TYPE)
            .zip(&self.trigger_configs)
    }

    /// Returns a new StoreBuilder for the given component ID.
    pub fn store_builder(
        &self,
        component_id: &str,
        wasi_version: WasiVersion,
    ) -> Result<StoreBuilder> {
        let mut builder = self.engine.store_builder(wasi_version);
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
    ) -> Result<(EitherInstance, Store<Executor::RuntimeData>)> {
        let store_builder = self.store_builder(component_id, WasiVersion::Preview2)?;
        self.prepare_instance_with_store(component_id, store_builder)
            .await
    }

    /// Returns a new Store and Instance for the given component ID and StoreBuilder.
    pub async fn prepare_instance_with_store(
        &self,
        component_id: &str,
        mut store_builder: StoreBuilder,
    ) -> Result<(EitherInstance, Store<Executor::RuntimeData>)> {
        let component = self.get_component(component_id)?;

        // Build Store
        component.apply_store_config(&mut store_builder).await?;
        let mut store = store_builder.build()?;

        // Instantiate
        let pre = self
            .component_instance_pres
            .get(component_id)
            .expect("component_instance_pres missing valid component_id");

        let instance = match pre {
            EitherInstancePre::Component(pre) => pre
                .instantiate_async(&mut store)
                .await
                .map(EitherInstance::Component),

            EitherInstancePre::Module(pre) => pre
                .instantiate_async(&mut store)
                .await
                .map(EitherInstance::Module),
        }
        .with_context(|| {
            format!(
                "app {:?} component {:?} instantiation failed",
                self.app_name, component_id
            )
        })?;

        Ok((instance, store))
    }

    pub fn get_component(&self, component_id: &str) -> Result<AppComponent> {
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
