use std::{
    collections::HashMap,
    error::Error,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use anyhow::Result;
use async_trait::async_trait;
use spin_config::{host_component::ConfigHostComponent, Resolver};
use spin_engine::{
    io::FollowComponents, Builder, Engine, ExecutionContext, ExecutionContextConfiguration,
};
use spin_manifest::{Application, ApplicationOrigin, ApplicationTrigger, TriggerConfig, Variable};

pub mod cli;
#[async_trait]
pub trait TriggerExecutor: Sized {
    type GlobalConfig;
    type TriggerConfig;
    type RunConfig;
    type RuntimeContext: Default + Send + 'static;

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

        // The .env file is either a sibling to the manifest file or (for bindles) in the current dir.
        let dotenv_root = match &app.info.origin {
            ApplicationOrigin::File(path) => path.parent().unwrap(),
            ApplicationOrigin::Bindle { .. } => Path::new("."),
        };

        // Build ExecutionContext
        let ctx_config = ExecutionContextConfiguration {
            components: app.components,
            label: app.info.name,
            log_dir: self.log_dir,
            follow_components: self.follow_components,
        };
        let engine = Engine::new(self.wasmtime_config)?;
        let mut ctx_builder = Builder::with_engine(ctx_config, engine)?;
        ctx_builder.link_defaults()?;
        if !self.disable_default_host_components {
            add_default_host_components(&mut ctx_builder)?;
            add_config_host_component(&mut ctx_builder, app.variables, dotenv_root)?;
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
pub fn add_default_host_components<T: Default + Send + 'static>(
    builder: &mut Builder<T>,
) -> Result<()> {
    builder.add_host_component(outbound_http::OutboundHttpComponent)?;
    builder.add_host_component(outbound_redis::OutboundRedis {
        connections: Arc::new(RwLock::new(HashMap::new())),
    })?;
    builder.add_host_component(outbound_pg::OutboundPg {
        connections: HashMap::new(),
    })?;
    Ok(())
}

pub fn add_config_host_component<T: Default + Send + 'static>(
    ctx_builder: &mut Builder<T>,
    variables: HashMap<String, Variable>,
    dotenv_path: &Path,
) -> Result<()> {
    let mut resolver = Resolver::new(variables)?;

    // Add all component configs to the Resolver.
    for component in &ctx_builder.config().components {
        resolver.add_component_config(
            &component.id,
            component.config.iter().map(|(k, v)| (k.clone(), v.clone())),
        )?;
    }

    let envs = read_dotenv(dotenv_path)?;

    // Add default config provider(s).
    // TODO(lann): Make config provider(s) configurable.
    resolver.add_provider(spin_config::provider::env::EnvProvider::new(
        spin_config::provider::env::DEFAULT_PREFIX,
        envs,
    ));

    ctx_builder.add_host_component(ConfigHostComponent::new(resolver))?;
    Ok(())
}

// Return environment key value mapping in ".env" file.
fn read_dotenv(dotenv_root: &Path) -> Result<HashMap<String, String>> {
    let dotenv_path = dotenv_root.join(".env");
    if !dotenv_path.is_file() {
        return Ok(Default::default());
    }
    dotenvy::from_path_iter(dotenv_path)?
        .map(|item| Ok(item?))
        .collect()
}
