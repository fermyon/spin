use std::{convert::TryFrom, error::Error, path::PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;
use spin_engine::{Builder, ExecutionContext, ExecutionContextConfiguration};
use spin_manifest::{Application, ApplicationTrigger, ComponentMap, CoreComponent, TriggerConfig};

#[async_trait]
pub trait Trigger: Sized {
    type ContextData: Default + 'static;
    type Config;
    type ComponentConfig;
    type RunConfig;

    fn new(
        execution_context: ExecutionContext<Self::ContextData>,
        config: Self::Config,
        component_configs: ComponentMap<Self::ComponentConfig>,
    ) -> Result<Self>;

    fn configure_execution_context(builder: &mut Builder<Self::ContextData>) -> Result<()> {
        configure_execution_context_defaults(builder)
    }

    async fn run(self, config: Self::RunConfig) -> Result<()>;
}

pub fn configure_execution_context_defaults<T: Default + 'static>(
    builder: &mut Builder<T>,
) -> Result<()> {
    builder.link_defaults()?;
    builder.add_host_component(wasi_outbound_http::OutboundHttpComponent)?;
    builder.add_host_component(outbound_redis::OutboundRedis)?;
    Ok(())
}

pub struct RunOptions<T: Trigger> {
    log_dir: Option<PathBuf>,
    trigger_run_config: T::RunConfig,
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
    let mut builder = Builder::<T::ContextData>::new(ExecutionContextConfiguration {
        components: app.components,
        label: app.info.name,
        log_dir: opts.log_dir,
        config_resolver: app.config_resolver,
    })?;
    T::configure_execution_context(&mut builder)?;
    let execution_ctx = builder.build().await?;
    let config = app.info.trigger.try_into()?;
    let component_configs = app.component_triggers.try_map_values(|id, t| {
        t.clone()
            .try_into()
            .with_context(|| format!("Failed to get trigger config for component {}", id))
    })?;
    let trigger = T::new(execution_ctx, config, component_configs)?;
    trigger.run(opts.trigger_run_config).await
}
