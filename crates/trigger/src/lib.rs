use std::{error::Error, path::PathBuf, sync::Arc};

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
    /// trigger extra
    type TriggerExtra;

    fn new(
        execution_context: ExecutionContext<Self::ContextData>,
        config: Self::Config,
        component_configs: ComponentMap<Self::ComponentConfig>,
        trigger_extra: Self::TriggerExtra,
    ) -> Result<Self>;

    fn build_trigger_extra(app: Application<CoreComponent>) -> Result<Self::TriggerExtra>;
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

pub async fn get_default_trigger<T: Trigger>(
    app: Application<CoreComponent>,
    log_dir: Option<PathBuf>,
) -> Result<T>
where
    T::Config: TryFrom<ApplicationTrigger>,
    T::ComponentConfig: TryFrom<TriggerConfig>,
    <T::Config as TryFrom<ApplicationTrigger>>::Error: Error + Send + Sync + 'static,
    <T::ComponentConfig as TryFrom<TriggerConfig>>::Error: Error + Send + Sync + 'static,
{
    let app_2 = app.clone();
    let mut builder = Builder::<T::ContextData>::new(ExecutionContextConfiguration {
        components: app_2.components,
        label: app_2.info.name,
        log_dir: log_dir,
        config_resolver: app_2.config_resolver,
    })?;
    T::configure_execution_context(&mut builder)?;
    let execution_ctx = builder.build().await?;
    let trigger_config = app_2.info.trigger.try_into()?;

    let component_triggers = app_2.component_triggers.try_map_values(|id, trigger| {
        trigger
            .clone()
            .try_into()
            .with_context(|| format!("Failed to convert trigger config for component {}", id))
    })?;

    let trigger_extra = T::build_trigger_extra(app)?;
    let trigger = T::new(
        execution_ctx,
        trigger_config,
        component_triggers,
        trigger_extra,
    );
    trigger
}

pub async fn run_trigger<T: Trigger>(
    app: Application<CoreComponent>,
    opts: RunOptions<T>,
) -> Result<()>
where
    T::Config: TryFrom<ApplicationTrigger>,
    T::ComponentConfig: TryFrom<TriggerConfig>,
    <T::Config as TryFrom<ApplicationTrigger>>::Error: Error + Send + Sync + 'static,
    <T::ComponentConfig as TryFrom<TriggerConfig>>::Error: Error + Send + Sync + 'static,
{
    let trigger: T = get_default_trigger(app, opts.log_dir).await?;
    trigger.run(opts.trigger_run_config).await
}
