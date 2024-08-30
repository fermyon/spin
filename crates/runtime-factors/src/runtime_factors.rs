use std::path::{Path, PathBuf};

use super::{TriggerAppOptions, TriggerFactors, TriggerFactorsRuntimeConfig};

use anyhow::Context as _;
use spin_common::ui::quoted_path;
use spin_factors_executor::FactorsExecutor;
use spin_runtime_config::ResolvedRuntimeConfig;
use spin_trigger::cli::{
    CommonTriggerOptions, InitialKvSetterHook, KeyValueDefaultStoreSummaryHook,
    RuntimeFactorsBuilder, SqlStatementExecutorHook, SqliteDefaultStoreSummaryHook,
    StdioLoggingExecutorHooks,
};
use toml::Value;

/// A [`RuntimeFactorsBuilder`] for [`TriggerFactors`].
pub struct FactorsBuilder;

impl RuntimeFactorsBuilder for FactorsBuilder {
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

        summarize_runtime_config(
            &runtime_config,
            common_options.runtime_config_file.as_deref(),
        );

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
        executor.add_hooks(StdioLoggingExecutorHooks::new(
            common_options.follow_components.clone(),
            runtime_config.log_dir(),
        ));
        executor.add_hooks(SqlStatementExecutorHook::new(
            options.sqlite_statements.clone(),
        ));
        executor.add_hooks(InitialKvSetterHook::new(options.key_values.clone()));
        executor.add_hooks(SqliteDefaultStoreSummaryHook);
        executor.add_hooks(KeyValueDefaultStoreSummaryHook);
        Ok(())
    }
}

fn summarize_runtime_config<T>(
    runtime_config: &ResolvedRuntimeConfig<T>,
    runtime_config_path: Option<&Path>,
) {
    let toml = &runtime_config.toml;
    let summarize_labeled_typed_tables = |key| {
        let mut summaries = vec![];
        if let Some(tables) = toml.get(key).and_then(Value::as_table) {
            for (label, config) in tables {
                if let Some(ty) = config.get("type").and_then(Value::as_str) {
                    summaries.push(format!("[{key}.{label}: {ty}]"))
                }
            }
        }
        summaries
    };

    let mut summaries = vec![];
    // [key_value_store.<label>: <type>]
    summaries.extend(summarize_labeled_typed_tables("key_value_store"));
    // [sqlite_database.<label>: <type>]
    summaries.extend(summarize_labeled_typed_tables("sqlite_database"));
    // [llm_compute: <type>]
    if let Some(table) = toml.get("llm_compute").and_then(Value::as_table) {
        if let Some(ty) = table.get("type").and_then(Value::as_str) {
            summaries.push(format!("[llm_compute: {ty}"));
        }
    }
    if !summaries.is_empty() {
        let summaries = summaries.join(", ");
        let from_path = runtime_config_path
            .map(|path| format!("from {}", quoted_path(path)))
            .unwrap_or_default();
        println!("Using runtime config {summaries} {from_path}");
    }
}
