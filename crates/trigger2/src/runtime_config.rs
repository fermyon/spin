use std::path::PathBuf;

use spin_factor_key_value::{self as key_value, KeyValueFactor};
use spin_factor_wasi::WasiFactor;
use spin_factors::{
    runtime_config::toml::TomlKeyTracker, FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer,
};

use crate::factors::TriggerFactorsRuntimeConfig;

/// A runtime configuration source for the [`TriggerFactors`][crate::TriggerFactors].
pub struct RuntimeConfigSource<'a> {
    table: TomlKeyTracker<'a>,
    pub key_value: &'a key_value::runtime_config::spin::RuntimeConfigResolver,
}

impl<'a> RuntimeConfigSource<'a> {
    pub fn new(
        table: &'a toml::Table,
        key_value: &'a key_value::runtime_config::spin::RuntimeConfigResolver,
    ) -> Self {
        Self {
            table: TomlKeyTracker::new(table),
            key_value,
        }
    }
}

impl RuntimeConfigSourceFinalizer for RuntimeConfigSource<'_> {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(self.table.validate_all_keys_used()?)
    }
}

impl FactorRuntimeConfigSource<WasiFactor> for RuntimeConfigSource<'_> {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<()>> {
        Ok(None)
    }
}

impl FactorRuntimeConfigSource<KeyValueFactor> for RuntimeConfigSource<'_> {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<spin_factor_key_value::RuntimeConfig>> {
        self.key_value.resolve_from_toml(Some(self.table.as_ref()))
    }
}

impl TryFrom<RuntimeConfigSource<'_>> for TriggerFactorsRuntimeConfig {
    type Error = anyhow::Error;

    fn try_from(value: RuntimeConfigSource<'_>) -> Result<Self, Self::Error> {
        Self::from_source(value)
    }
}

const DEFAULT_SPIN_STORE_FILENAME: &str = "sqlite_key_value.db";

/// The key-value runtime configuration resolver used by the trigger.
///
/// Takes a base path for the local store.
pub fn key_value_resolver(
    local_store_base_path: PathBuf,
) -> spin_factor_key_value::runtime_config::spin::RuntimeConfigResolver {
    let mut key_value = key_value::runtime_config::spin::RuntimeConfigResolver::new();
    key_value.add_default_store(
        "default",
        spin_factor_key_value::runtime_config::spin::StoreConfig {
            type_: "spin".to_owned(),
            config: toml::toml! {
                path = DEFAULT_SPIN_STORE_FILENAME
            },
        },
    );
    // Unwraps are safe because the store types are known to not overlap.
    key_value
        .register_store_type(spin_factor_key_value_spin::SpinKeyValueStore::new(
            local_store_base_path,
        ))
        .unwrap();
    key_value
        .register_store_type(spin_factor_key_value_redis::RedisKeyValueStore::new())
        .unwrap();
    key_value
        .register_store_type(spin_factor_key_value_azure::AzureKeyValueStore::new())
        .unwrap();
    key_value
}
