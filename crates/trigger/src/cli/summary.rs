use std::path::Path;

use spin_common::ui::quoted_path;
use spin_core::async_trait;
use spin_factor_key_value::KeyValueFactor;
use spin_factor_sqlite::SqliteFactor;
use spin_factors::RuntimeFactors;
use spin_factors_executor::ExecutorHooks;
use spin_runtime_config::ResolvedRuntimeConfig;
use toml::Value;

use crate::factors::TriggerFactors;

pub fn summarize_runtime_config<T>(
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

/// An [`ExecutorHooks`] that prints information about the default KV store.
pub struct KeyValueDefaultStoreSummaryHook;

#[async_trait]
impl<F: RuntimeFactors, U> ExecutorHooks<F, U> for KeyValueDefaultStoreSummaryHook {
    async fn configure_app(
        &mut self,
        configured_app: &spin_factors::ConfiguredApp<F>,
    ) -> anyhow::Result<()> {
        let Ok(kv_app_state) = configured_app.app_state::<KeyValueFactor>() else {
            return Ok(());
        };
        if !kv_app_state.store_is_used("default") {
            // We don't talk about unused default stores
            return Ok(());
        }
        if let Some(default_store_summary) = kv_app_state.store_summary("default") {
            println!("Storing default key-value data to {default_store_summary}.");
        }
        Ok(())
    }
}

/// An [`ExecutorHooks`] that prints information about the default KV store.
pub struct SqliteDefaultStoreSummaryHook;

#[async_trait]
impl<U> ExecutorHooks<TriggerFactors, U> for SqliteDefaultStoreSummaryHook {
    async fn configure_app(
        &mut self,
        configured_app: &spin_factors::ConfiguredApp<TriggerFactors>,
    ) -> anyhow::Result<()> {
        let Ok(sqlite_app_state) = configured_app.app_state::<SqliteFactor>() else {
            return Ok(());
        };
        if !sqlite_app_state.database_is_used("default") {
            // We don't talk about unused default databases
            return Ok(());
        }
        if let Some(default_database_summary) = sqlite_app_state
            .get_connection("default")
            .and_then(Result::ok)
            .and_then(|conn| conn.summary())
        {
            println!("Storing default SQLite data to {default_database_summary}.");
        }
        Ok(())
    }
}
