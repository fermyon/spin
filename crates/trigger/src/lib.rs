use std::{error::Error, marker::PhantomData, path::PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use spin_engine::{
    io::FollowComponents, Builder, Engine, ExecutionContext, ExecutionContextConfiguration,
};
use spin_manifest::{Application, ApplicationTrigger, TriggerConfig};

pub mod cli;
#[async_trait]
pub trait TriggerExecutor: Sized {
    type GlobalConfig;
    type TriggerConfig;
    type RunConfig;
    type RuntimeContext: Default + 'static;

    /// Create a new trigger executor.
    fn new(
        execution_context: ExecutionContext<Self::RuntimeContext>,
        global_config: Self::GlobalConfig,
        trigger_configs: impl IntoIterator<Item = Self::TriggerConfig>,
    ) -> Result<Self>;

    /// Run the trigger executor.
    async fn run(self, config: Self::RunConfig) -> Result<()>;

    /// Make changes to the ExecutionContext using the given Builder.
    fn configure_execution_context(_builder: &mut Builder<Self::RuntimeContext>) -> Result<()> {
        Ok(())
    }
}

pub struct TriggerExecutorBuilder<Executor: TriggerExecutor> {
    application: Application,
    wasmtime_config: wasmtime::Config,
    log_dir: Option<PathBuf>,
    follow_components: FollowComponents,
    disable_default_host_components: bool,
    _phantom: PhantomData<Executor>,
}

impl<Executor: TriggerExecutor> TriggerExecutorBuilder<Executor> {
    /// Create a new TriggerExecutorBuilder with the given Application.
    pub fn new(application: Application) -> Self {
        Self {
            application,
            wasmtime_config: Default::default(),
            log_dir: None,
            follow_components: Default::default(),
            disable_default_host_components: false,
            _phantom: PhantomData,
        }
    }

    /// !!!Warning!!! Using a custom Wasmtime Config is entirely unsupported;
    /// many configurations are likely to cause errors or unexpected behavior.
    #[doc(hidden)]
    pub fn wasmtime_config_mut(&mut self) -> &mut wasmtime::Config {
        &mut self.wasmtime_config
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

    pub async fn build(self) -> Result<Executor>
    where
        Executor::GlobalConfig: TryFrom<ApplicationTrigger>,
        <Executor::GlobalConfig as TryFrom<ApplicationTrigger>>::Error:
            Error + Send + Sync + 'static,
        Executor::TriggerConfig: TryFrom<(String, TriggerConfig)>,
        <Executor::TriggerConfig as TryFrom<(String, TriggerConfig)>>::Error:
            Error + Send + Sync + 'static,
    {
        let app = self.application;

        // Build ExecutionContext
        let ctx_config = ExecutionContextConfiguration {
            components: app.components,
            label: app.info.name,
            log_dir: self.log_dir,
            follow_components: self.follow_components,
            config_resolver: app.config_resolver,
            module_io_redirects: Default::default(),
        };
        let engine = Engine::new(self.wasmtime_config)?;
        let mut ctx_builder = Builder::with_engine(ctx_config, engine)?;
        ctx_builder.link_defaults()?;
        if !self.disable_default_host_components {
            add_default_host_components(&mut ctx_builder)?;
        }
        Executor::configure_execution_context(&mut ctx_builder)?;
        let execution_context = ctx_builder.build().await?;

        // Build trigger configurations
        let global_config = app.info.trigger.try_into()?;
        let trigger_configs = app
            .component_triggers
            .into_iter()
            .map(|(id, config)| Ok((id, config).try_into()?))
            .collect::<Result<Vec<_>>>()?;

        // Run trigger executor
        Executor::new(execution_context, global_config, trigger_configs)
    }
}

/// Add the default set of host components to the given builder.
pub fn add_default_host_components<T: Default + 'static>(builder: &mut Builder<T>) -> Result<()> {
    builder.add_host_component(wasi_outbound_http::OutboundHttpComponent)?;
    builder.add_host_component(outbound_redis::OutboundRedis)?;
    builder.add_host_component(outbound_pg::OutboundPg)?;
    Ok(())
}
