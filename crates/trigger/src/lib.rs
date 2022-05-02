use std::{error::Error, path::PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;
use spin_engine::{Builder, ExecutionContext, ExecutionContextConfiguration};
use spin_manifest::{Application, ApplicationTrigger, ComponentMap, CoreComponent, TriggerConfig};

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
    type RuntimeConfig;

    fn new(
        execution_context: ExecutionContext<Self::ContextData>,
        config: Self::Config,
        component_configs: ComponentMap<Self::ComponentConfig>,
    ) -> Result<Self>;

    async fn run(&self, run_config: Self::RuntimeConfig) -> Result<()>;

    fn configure_execution_context(builder: &mut Builder<Self::ContextData>) -> Result<()> {
        builder.link_defaults()?;
        builder.add_host_component(wasi_outbound_http::OutboundHttpComponent)?;
        builder.add_host_component(outbound_redis::OutboundRedis)?;
        Ok(())
    }
}

pub struct RunOptions<T: Trigger> {
    log_dir: Option<PathBuf>,
    trigger_run_config: T::RuntimeConfig,
}

impl<T: Trigger> RunOptions<T> {
    pub fn new(log_dir: Option<PathBuf>, trigger_run_config: T::RuntimeConfig) -> Self {
        Self {
            log_dir,
            trigger_run_config,
        }
    }
}

pub async fn build_trigger_from_app<T: Trigger>(
    app: Application<CoreComponent>,
    log_dir: Option<PathBuf>,
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
        log_dir,
        config_resolver: app.config_resolver,
    };
    let mut builder = match wasmtime_config {
        Some(wasmtime_config) => {
            Builder::<T::ContextData>::with_wasmtime_config(config, wasmtime_config)
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

    T::new(execution_ctx, trigger_config, component_triggers)
}

pub async fn run_trigger<T: Trigger>(
    app: Application<CoreComponent>,
    opts: RunOptions<T>,
    wasmtime_config: Option<wasmtime::Config>,
) -> Result<()>
where
    T::Config: TryFrom<ApplicationTrigger>,
    T::ComponentConfig: TryFrom<TriggerConfig>,
    <T::Config as TryFrom<ApplicationTrigger>>::Error: Error + Send + Sync + 'static,
    <T::ComponentConfig as TryFrom<TriggerConfig>>::Error: Error + Send + Sync + 'static,
{
    let trigger: T = build_trigger_from_app(app, opts.log_dir, wasmtime_config).await?;
    trigger.run(opts.trigger_run_config).await
}
