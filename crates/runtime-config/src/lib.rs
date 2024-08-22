use std::path::{Path, PathBuf};

use anyhow::Context as _;
use spin_factor_key_value::runtime_config::spin::{self as key_value, MakeKeyValueStore};
use spin_factor_key_value::{DefaultLabelResolver as _, KeyValueFactor};
use spin_factor_outbound_http::OutboundHttpFactor;
use spin_factor_outbound_mqtt::OutboundMqttFactor;
use spin_factor_outbound_networking::runtime_config::spin::SpinTlsRuntimeConfig;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_outbound_pg::OutboundPgFactor;
use spin_factor_outbound_redis::OutboundRedisFactor;
use spin_factor_sqlite::runtime_config::spin as sqlite;
use spin_factor_sqlite::SqliteFactor;
use spin_factor_variables::{spin_cli as variables, VariablesFactor};
use spin_factor_wasi::WasiFactor;
use spin_factors::{
    runtime_config::toml::TomlKeyTracker, FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer,
};

/// The default state directory for the trigger.
pub const DEFAULT_STATE_DIR: &str = ".spin";

/// A runtime configuration which has been resolved from a runtime config source.
///
/// Includes other pieces of configuration that are used to resolve the runtime configuration.
pub struct ResolvedRuntimeConfig<T> {
    /// The resolved runtime configuration.
    pub runtime_config: T,
    /// The resolver used to resolve key-value stores from runtime configuration.
    pub key_value_resolver: key_value::RuntimeConfigResolver,
    /// The resolver used to resolve sqlite databases from runtime configuration.
    pub sqlite_resolver: sqlite::RuntimeConfigResolver,
}

impl<T> ResolvedRuntimeConfig<T>
where
    T: for<'a> TryFrom<TomlRuntimeConfigSource<'a>>,
    for<'a> <T as TryFrom<TomlRuntimeConfigSource<'a>>>::Error: Into<anyhow::Error>,
{
    /// Creates a new resolved runtime configuration from a runtime config source TOML file.
    pub fn from_file(runtime_config_path: &Path, state_dir: Option<&str>) -> anyhow::Result<Self> {
        let tls_resolver = SpinTlsRuntimeConfig::new(runtime_config_path);
        let key_value_config_resolver =
            key_value_config_resolver(PathBuf::from(state_dir.unwrap_or(DEFAULT_STATE_DIR)));

        let sqlite_config_resolver =
            sqlite_config_resolver(state_dir).context("failed to resolve sqlite runtime config")?;

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
        let runtime_config: T = TomlRuntimeConfigSource::new(
            &toml,
            &key_value_config_resolver,
            &tls_resolver,
            &sqlite_config_resolver,
        )
        .try_into()
        .map_err(Into::into)?;

        Ok(Self {
            runtime_config,
            key_value_resolver: key_value_config_resolver,
            sqlite_resolver: sqlite_config_resolver,
        })
    }

    /// Set initial key-value pairs supplied in the CLI arguments in the default store.
    pub async fn set_initial_key_values(
        &self,
        initial_key_values: impl IntoIterator<Item = &(String, String)>,
    ) -> anyhow::Result<()> {
        let store = self
            .key_value_resolver
            .default(DEFAULT_KEY_VALUE_STORE_LABEL)
            .expect("trigger was misconfigured and lacks a default store")
            .get(DEFAULT_KEY_VALUE_STORE_LABEL)
            .await
            .expect("trigger was misconfigured and lacks a default store");
        for (key, value) in initial_key_values {
            store
                .set(key, value.as_bytes())
                .await
                .context("failed to set key-value pair")?;
        }
        Ok(())
    }
}

impl<T: Default> ResolvedRuntimeConfig<T> {
    pub fn default(state_dir: Option<&str>) -> Self {
        Self {
            sqlite_resolver: sqlite_config_resolver(state_dir)
                .expect("failed to resolve sqlite runtime config"),
            key_value_resolver: key_value_config_resolver(PathBuf::from(
                state_dir.unwrap_or(DEFAULT_STATE_DIR),
            )),
            runtime_config: Default::default(),
        }
    }
}

/// The TOML based runtime configuration source Spin CLI.
pub struct TomlRuntimeConfigSource<'a> {
    table: TomlKeyTracker<'a>,
    key_value: &'a key_value::RuntimeConfigResolver,
    tls: &'a SpinTlsRuntimeConfig,
    sqlite: &'a sqlite::RuntimeConfigResolver,
}

impl<'a> TomlRuntimeConfigSource<'a> {
    pub fn new(
        table: &'a toml::Table,
        key_value: &'a key_value::RuntimeConfigResolver,
        tls: &'a SpinTlsRuntimeConfig,
        sqlite: &'a sqlite::RuntimeConfigResolver,
    ) -> Self {
        Self {
            table: TomlKeyTracker::new(table),
            key_value,
            tls,
            sqlite,
        }
    }
}

impl FactorRuntimeConfigSource<KeyValueFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<spin_factor_key_value::RuntimeConfig>> {
        self.key_value.resolve_from_toml(Some(self.table.as_ref()))
    }
}

impl FactorRuntimeConfigSource<OutboundNetworkingFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<<OutboundNetworkingFactor as spin_factors::Factor>::RuntimeConfig>>
    {
        self.tls.config_from_table(self.table.as_ref())
    }
}

impl FactorRuntimeConfigSource<VariablesFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<<VariablesFactor as spin_factors::Factor>::RuntimeConfig>> {
        Ok(Some(variables::runtime_config_from_toml(
            self.table.as_ref(),
        )?))
    }
}

impl FactorRuntimeConfigSource<OutboundPgFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<()>> {
        Ok(None)
    }
}

impl FactorRuntimeConfigSource<OutboundRedisFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<()>> {
        Ok(None)
    }
}

impl FactorRuntimeConfigSource<WasiFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<()>> {
        Ok(None)
    }
}

impl FactorRuntimeConfigSource<OutboundHttpFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<()>> {
        Ok(None)
    }
}

impl FactorRuntimeConfigSource<OutboundMqttFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<()>> {
        Ok(None)
    }
}

impl FactorRuntimeConfigSource<SqliteFactor> for TomlRuntimeConfigSource<'_> {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<spin_factor_sqlite::RuntimeConfig>> {
        self.sqlite.resolve_from_toml(self.table.as_ref())
    }
}

impl RuntimeConfigSourceFinalizer for TomlRuntimeConfigSource<'_> {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(self.table.validate_all_keys_used()?)
    }
}

const DEFAULT_KEY_VALUE_STORE_FILENAME: &str = "sqlite_key_value.db";
const DEFAULT_KEY_VALUE_STORE_LABEL: &str = "default";

/// The key-value runtime configuration resolver.
///
/// Takes a base path for the local store.
pub fn key_value_config_resolver(
    local_store_base_path: PathBuf,
) -> key_value::RuntimeConfigResolver {
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
        DEFAULT_KEY_VALUE_STORE_LABEL,
        key_value::StoreConfig {
            type_: spin_factor_key_value_spin::SpinKeyValueStore::RUNTIME_CONFIG_TYPE.to_owned(),
            config: toml::toml! {
                path = DEFAULT_KEY_VALUE_STORE_FILENAME
            },
        },
    );

    key_value
}

/// The sqlite runtime configuration resolver.
///
/// Takes a base path to the state directory.
fn sqlite_config_resolver(
    state_dir: Option<&str>,
) -> anyhow::Result<sqlite::RuntimeConfigResolver> {
    let default_database_dir = PathBuf::from(state_dir.unwrap_or(DEFAULT_STATE_DIR));
    let local_database_dir =
        std::env::current_dir().context("failed to get current working directory")?;
    Ok(sqlite::RuntimeConfigResolver::new(
        default_database_dir,
        local_database_dir,
    ))
}
