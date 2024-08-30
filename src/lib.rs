pub mod build_info;
pub mod commands;
pub(crate) mod opts;
pub mod subprocess;

use std::path::PathBuf;

use anyhow::Context as _;
pub use opts::HELP_ARGS_ONLY_TRIGGER_TYPE;
use spin_factors_executor::FactorsExecutor;
use spin_runtime_config::ResolvedRuntimeConfig;
use spin_trigger::{
    cli::{
        CommonTriggerOptions, KeyValueDefaultStoreSummaryHook, RuntimeFactorsBuilder,
        SqlStatementExecutorHook, StdioLoggingExecutorHooks,
    },
    TriggerAppOptions, TriggerFactors, TriggerFactorsRuntimeConfig,
};

pub struct Builder;

impl RuntimeFactorsBuilder for Builder {
    type Options = TriggerAppOptions;
    type Factors = TriggerFactors;
    type RuntimeConfig = ResolvedRuntimeConfig<TriggerFactorsRuntimeConfig>;

    fn build(
        common_options: &CommonTriggerOptions,
        options: &Self::Options,
    ) -> anyhow::Result<(Self::Factors, Self::RuntimeConfig)> {
        // Hardcode `use_gpu` to true for now
        let use_gpu = true;
        let runtime_config = ResolvedRuntimeConfig::<TriggerFactorsRuntimeConfig>::from_file(
            common_options.runtime_config_file.clone().as_deref(),
            common_options.local_app_dir.clone().map(PathBuf::from),
            common_options.state_dir.clone(),
            common_options.log_dir.clone(),
            use_gpu,
        )?;

        let factors = TriggerFactors::new(
            runtime_config.state_dir(),
            common_options.working_dir.clone(),
            options.allow_transient_write,
            runtime_config.key_value_resolver.clone(),
            runtime_config.sqlite_resolver.clone(),
            use_gpu,
        )
        .context("failed to create factors")?;
        Ok((factors, runtime_config))
    }

    fn configure_app<U: Send + 'static>(
        executor: &mut FactorsExecutor<Self::Factors, U>,
        runtime_config: &Self::RuntimeConfig,
        common_options: &CommonTriggerOptions,
        options: &Self::Options,
    ) -> anyhow::Result<()> {
        executor.add_hooks(SqlStatementExecutorHook::new(
            options.sqlite_statements.clone(),
        ));
        executor.add_hooks(StdioLoggingExecutorHooks::new(
            common_options.follow_components.clone(),
            runtime_config.log_dir(),
        ));
        executor.add_hooks(KeyValueDefaultStoreSummaryHook);
        // TODO: implement initial key values as a hook
        // runtime_config
        //     .set_initial_key_values(&options.initial_key_values)
        //     .await?;
        // builder.hooks(SummariseRuntimeConfigHook::new(&self.runtime_config_file));
        // builder.hooks(SqlitePersistenceMessageHook);
        Ok(())
    }
}
