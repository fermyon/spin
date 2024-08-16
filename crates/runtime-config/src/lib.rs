use std::path::{Path, PathBuf};

use anyhow::Context as _;
use spin_factor_key_value::runtime_config::spin::{self as key_value, MakeKeyValueStore};
use spin_factor_key_value::KeyValueFactor;
use spin_factor_wasi::WasiFactor;
use spin_factors::{
    runtime_config::toml::TomlKeyTracker, FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer,
};

pub const DEFAULT_STATE_DIR: &str = ".spin";

/// A runtime configuration which has been resolved from a runtime config source.
///
/// Includes other pieces of configuration that are used to resolve the runtime configuration.
#[derive(Default)]
pub struct ResolvedRuntimeConfig<T> {
    /// The resolved runtime configuration.
    pub runtime_config: T,
    /// The resolver used to resolve key-value stores from runtime configuration.
    pub key_value_resolver: key_value::RuntimeConfigResolver,
}

impl<T> ResolvedRuntimeConfig<T>
where
    T: for<'a> TryFrom<TomlRuntimeConfigSource<'a>>,
    for<'a> <T as TryFrom<TomlRuntimeConfigSource<'a>>>::Error: Into<anyhow::Error>,
{
    /// Creates a new resolved runtime configuration from a runtime config source TOML file.
    pub fn from_file(runtime_config_path: &Path, state_dir: Option<&str>) -> anyhow::Result<Self> {
        let key_value_resolver = key_value_resolver(PathBuf::from(
            state_dir.unwrap_or_else(|| DEFAULT_STATE_DIR.into()),
        ));

        let file = std::fs::read_to_string(runtime_config_path).with_context(|| {
            format!(
                "failed to read runtime config file '{}'",
                runtime_config_path.display()
            )
        })?;
        let toml = toml::from_str(&file).with_context(|| {
            format!(
                "failed to parse runtime config file '{}' as toml",
                runtime_config_path.display()
            )
        })?;
        let runtime_config: T = TomlRuntimeConfigSource::new(&toml, &key_value_resolver)
            .try_into()
            .map_err(Into::into)?;

        Ok(Self {
            runtime_config,
            key_value_resolver,
        })
    }
}

/// The TOML based runtime configuration source Spin CLI.
pub struct TomlRuntimeConfigSource<'a> {
    table: TomlKeyTracker<'a>,
    key_value: &'a key_value::RuntimeConfigResolver,
}

impl<'a> TomlRuntimeConfigSource<'a> {
    pub fn new(table: &'a toml::Table, key_value: &'a key_value::RuntimeConfigResolver) -> Self {
        Self {
            table: TomlKeyTracker::new(table),
            key_value,
        }
    }
}

impl FactorRuntimeConfigSource<WasiFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<()>> {
        Ok(None)
    }
}

impl FactorRuntimeConfigSource<KeyValueFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<spin_factor_key_value::RuntimeConfig>> {
        self.key_value.resolve_from_toml(Some(self.table.as_ref()))
    }
}

impl RuntimeConfigSourceFinalizer for TomlRuntimeConfigSource<'_> {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(self.table.validate_all_keys_used()?)
    }
}

const DEFAULT_SPIN_STORE_FILENAME: &str = "sqlite_key_value.db";

/// The key-value runtime configuration resolver used by the trigger.
///
/// Takes a base path for the local store.
pub fn key_value_resolver(local_store_base_path: PathBuf) -> key_value::RuntimeConfigResolver {
    let mut key_value = key_value::RuntimeConfigResolver::new();

    // Register the supported store types.
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

    // Add handling of "default" store.
    key_value.add_default_store(
        "default",
        key_value::StoreConfig {
            type_: spin_factor_key_value_spin::SpinKeyValueStore::RUNTIME_CONFIG_TYPE.to_owned(),
            config: toml::toml! {
                path = DEFAULT_SPIN_STORE_FILENAME
            },
        },
    );

    key_value
}
