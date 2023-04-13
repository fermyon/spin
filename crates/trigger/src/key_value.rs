use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use spin_key_value::{
    CachingStoreManager, DelegatingStoreManager, KeyValueComponent, KEY_VALUE_STORES_KEY,
};
use spin_key_value_sqlite::{DatabaseLocation, KeyValueSqlite};

use crate::{runtime_config::RuntimeConfig, TriggerHooks};

// TODO: Once we have runtime configuration for key-value stores, the user will be able
// to both change the default store configuration (e.g. use Redis, or an SQLite
// in-memory database, or use a different path) and add other named stores with their
// own configurations.

pub(crate) fn build_key_value_component(
    runtime_config: &RuntimeConfig,
) -> Result<KeyValueComponent> {
    let location = match runtime_config.key_value_sqlite_db_path() {
        Some(path) => {
            // Create the store's parent directory if necessary
            create_parent_dir(&path).context("Failed to create key value store")?;
            DatabaseLocation::Path(path)
        }
        None => DatabaseLocation::InMemory,
    };

    let manager = Arc::new(CachingStoreManager::new(DelegatingStoreManager::new([(
        "default".to_owned(),
        Arc::new(KeyValueSqlite::new(location)) as _,
    )])));

    Ok(KeyValueComponent::new(spin_key_value::manager(move |_| {
        manager.clone()
    })))
}

fn create_parent_dir(path: &Path) -> Result<()> {
    let dir = path
        .parent()
        .with_context(|| format!("{path:?} missing parent dir"))?;
    std::fs::create_dir_all(dir).with_context(|| format!("Failed to create parent dir {dir:?}"))
}

pub struct KeyValuePersistenceMessageHook;

impl TriggerHooks for KeyValuePersistenceMessageHook {
    fn app_loaded(&mut self, app: &spin_app::App, runtime_config: &RuntimeConfig) -> Result<()> {
        // Don't print anything if the app doesn't use KV
        if app.components().all(|c| {
            c.get_metadata(KEY_VALUE_STORES_KEY)
                .unwrap_or_default()
                .unwrap_or_default()
                .is_empty()
        }) {
            return Ok(());
        }
        if let Some(path) = runtime_config.key_value_sqlite_db_path() {
            println!("Storing key-value data to {path:?}");
        } else {
            println!("Using in-memory key-value store; data will not be saved!");
        }
        Ok(())
    }
}
