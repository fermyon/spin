use std::{error::Error, path::PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;
use spin_engine::{
    io::{FollowComponents, ModuleIoRedirectsTypes},
    Builder, Engine, ExecutionContext, ExecutionContextConfiguration,
};
use spin_manifest::{Application, ApplicationTrigger, ComponentMap, TriggerConfig};

/// The trigger
#[async_trait]
pub trait Trigger: Sized {
    /// data
    type ContextData: Default + 'static;
    /// trigger configuration
    type Config;
    /// component configuration
    type ComponentConfig;
    /// runtime configuration
    type ExecutionConfig;

    fn new(
        execution_context: ExecutionContext<Self::ContextData>,
        config: Self::Config,
        component_configs: ComponentMap<Self::ComponentConfig>,
        follow: FollowComponents,
    ) -> Result<Self>;

    async fn run(&self, run_config: Self::ExecutionConfig) -> Result<()>;

    fn configure_execution_context(builder: &mut Builder<Self::ContextData>) -> Result<()> {
        builder.link_defaults()?;
        builder.add_host_component(wasi_outbound_http::OutboundHttpComponent)?;
        builder.add_host_component(outbound_redis::OutboundRedis)?;
        Ok(())
    }
}

pub struct ExecutionOptions<T: Trigger> {
    kv_dir: Option<PathBuf>,
    log_dir: Option<PathBuf>,
    follow: FollowComponents,
    execution_config: T::ExecutionConfig,
}

impl<T: Trigger> ExecutionOptions<T> {
    pub fn new(
        kv_dir: Option<PathBuf>,
        log_dir: Option<PathBuf>,
        follow: FollowComponents,
        execution_config: T::ExecutionConfig,
    ) -> Self {
        Self {
            kv_dir,
            log_dir,
            follow,
            execution_config,
        }
    }
}

pub async fn build_trigger_from_app<T: Trigger>(
    app: Application,
    kv_dir: Option<PathBuf>,
    log_dir: Option<PathBuf>,
    follow: FollowComponents,
    wasmtime_config: Option<wasmtime::Config>,
) -> Result<T>
where
    T::Config: TryFrom<ApplicationTrigger>,
    T::ComponentConfig: TryFrom<TriggerConfig>,
    <T::Config as TryFrom<ApplicationTrigger>>::Error: Error + Send + Sync + 'static,
    <T::ComponentConfig as TryFrom<TriggerConfig>>::Error: Error + Send + Sync + 'static,
{
    let config = ExecutionContextConfiguration {
        components: app.components,
        label: app.info.name,
        kv_dir,
        log_dir,
        config_resolver: app.config_resolver,
        module_io_redirects: ModuleIoRedirectsTypes::default(),
    };
    let mut builder = match wasmtime_config {
        Some(wasmtime_config) => {
            Builder::<T::ContextData>::with_engine(config, Engine::new(wasmtime_config)?)
        }
        None => Builder::<T::ContextData>::new(config),
    }?;

    T::configure_execution_context(&mut builder)?;
    let execution_ctx = builder.build().await?;
    let trigger_config = app.info.trigger.try_into()?;

    let component_triggers = app
        .component_triggers
        .into_iter()
        .map(|(id, trigger)| {
            Ok((
                id.clone(),
                trigger.try_into().with_context(|| {
                    format!("Failed to convert trigger config for component {}", id)
                })?,
            ))
        })
        .collect::<Result<_>>()?;

    T::new(execution_ctx, trigger_config, component_triggers, follow)
}

pub async fn run_trigger<T: Trigger>(
    app: Application,
    opts: ExecutionOptions<T>,
    wasmtime_config: Option<wasmtime::Config>,
) -> Result<()>
where
    T::Config: TryFrom<ApplicationTrigger>,
    T::ComponentConfig: TryFrom<TriggerConfig>,
    <T::Config as TryFrom<ApplicationTrigger>>::Error: Error + Send + Sync + 'static,
    <T::ComponentConfig as TryFrom<TriggerConfig>>::Error: Error + Send + Sync + 'static,
{
    let trigger: T =
        build_trigger_from_app(app, opts.kv_dir, opts.log_dir, opts.follow, wasmtime_config)
            .await?;
    trigger.run(opts.execution_config).await
}
