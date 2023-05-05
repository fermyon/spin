use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::Arc,
};

use crate::{runtime_config::RuntimeConfig, TriggerHooks};
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use spin_key_value::{
    CachingStoreManager, DelegatingStoreManager, KeyValueComponent, StoreManager,
    KEY_VALUE_STORES_KEY,
};
use spin_key_value_sqlite::{DatabaseLocation, KeyValueSqlite};

use super::{resolve_config_path, RuntimeConfigOpts};

const DEFAULT_SPIN_STORE_FILENAME: &str = "sqlite_key_value.db";

pub type KeyValueStore = Arc<dyn StoreManager>;

/// Builds a [`KeyValueComponent`] from the given [`RuntimeConfig`].
pub async fn build_key_value_component(
    runtime_config: &RuntimeConfig,
    init_data: &[(String, String)],
) -> Result<(KeyValueComponent, impl TriggerHooks)> {
    let stores: HashMap<_, _> = runtime_config
        .key_value_stores()
        .context("Failed to build key-value component")?
        .into_iter()
        .collect();

    // Avoid creating a database as a side-effect if one is not needed.
    if !init_data.is_empty() {
        if let Some(manager) = stores.get("default") {
            let default_store = manager
                .get("default")
                .await
                .context("Failed to access key-value store to set requested entries")?;
            for (key, value) in init_data {
                default_store
                    .set(key, value.as_bytes())
                    .await
                    .with_context(|| {
                        format!("Failed to set requested entry {key} in key-value store")
                    })?;
            }
        } else {
            bail!("Failed to access key-value store to set requested entries");
        }
    }

    let delegating_manager = DelegatingStoreManager::new(stores);

    let store_names = delegating_manager.store_names().into_iter();
    let kv_hooks = KeyValueValidationHook::new(store_names);

    let caching_manager = Arc::new(CachingStoreManager::new(delegating_manager));
    let kv_component =
        KeyValueComponent::new(spin_key_value::manager(move |_| caching_manager.clone()));

    Ok((kv_component, kv_hooks))
}

struct KeyValueValidationHook {
    store_names: HashSet<String>,
}

impl KeyValueValidationHook {
    fn new(store_names: impl IntoIterator<Item = String>) -> Self {
        Self {
            store_names: store_names.into_iter().collect(),
        }
    }
}

impl TriggerHooks for KeyValueValidationHook {
    fn app_loaded(&mut self, app: &spin_app::App, _runtime_config: &RuntimeConfig) -> Result<()> {
        let errors  = app.components().flat_map(|c| {
            let allowed_stores = c.get_metadata(KEY_VALUE_STORES_KEY).unwrap_or_default().unwrap_or_default();
            allowed_stores.iter().filter_map(|allowed_store|
                if self.store_names.contains(allowed_store) {
                    None
                } else {
                    let err = format!("Component {} is granted access to key-value store '{allowed_store}', which is not defined", c.id());
                    Some(err)
                }
            ).collect::<Vec<String>>()
        }).collect::<Vec<String>>();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(errors.join("\n")))
        }
    }
}

// Holds deserialized options from a `[key_value_store.<name>]` runtime config section.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum KeyValueStoreOpts {
    Spin(SpinKeyValueStoreOpts),
    Redis(RedisKeyValueStoreOpts),
}

impl KeyValueStoreOpts {
    pub fn default_store_opts(runtime_config: &RuntimeConfig) -> Self {
        Self::Spin(SpinKeyValueStoreOpts::default_store_opts(runtime_config))
    }

    pub fn build_store(&self, config_opts: &RuntimeConfigOpts) -> Result<KeyValueStore> {
        match self {
            Self::Spin(opts) => opts.build_store(config_opts),
            Self::Redis(opts) => opts.build_store(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpinKeyValueStoreOpts {
    pub path: Option<PathBuf>,
}

impl SpinKeyValueStoreOpts {
    fn default_store_opts(runtime_config: &RuntimeConfig) -> Self {
        // If the state dir is set, build the default path
        let path = runtime_config
            .state_dir()
            .map(|dir| dir.join(DEFAULT_SPIN_STORE_FILENAME));
        Self { path }
    }

    fn build_store(&self, config_opts: &RuntimeConfigOpts) -> Result<KeyValueStore> {
        let location = match self.path.as_ref() {
            Some(path) => {
                let path = resolve_config_path(path, config_opts)?;
                // Create the store's parent directory if necessary
                fs::create_dir_all(path.parent().unwrap())
                    .context("Failed to create key value store")?;
                DatabaseLocation::Path(path)
            }
            None => DatabaseLocation::InMemory,
        };
        Ok(Arc::new(KeyValueSqlite::new(location)))
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RedisKeyValueStoreOpts {
    pub url: String,
}

impl RedisKeyValueStoreOpts {
    fn build_store(&self) -> Result<KeyValueStore> {
        let kv_redis = spin_key_value_redis::KeyValueRedis::new(self.url.clone())?;
        Ok(Arc::new(kv_redis))
    }
}

// Prints startup messages about the default key value store config.
pub struct KeyValuePersistenceMessageHook;

impl TriggerHooks for KeyValuePersistenceMessageHook {
    fn app_loaded(&mut self, app: &spin_app::App, runtime_config: &RuntimeConfig) -> Result<()> {
        // Only print if the app actually uses KV
        if app.components().all(|c| {
            c.get_metadata(KEY_VALUE_STORES_KEY)
                .unwrap_or_default()
                .unwrap_or_default()
                .is_empty()
        }) {
            return Ok(());
        }
        match runtime_config.default_key_value_opts() {
            KeyValueStoreOpts::Redis(_store_opts) => {
                println!("Storing default key-value data to Redis");
            }
            KeyValueStoreOpts::Spin(store_opts) => {
                if let Some(path) = &store_opts.path {
                    println!("Storing default key-value data to {path:?}");
                } else {
                    println!("Using in-memory default key-value store; data will not be saved!");
                }
            }
        }
        Ok(())
    }
}
