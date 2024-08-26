use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use spin_factor_key_value::runtime_config::spin::MakeKeyValueStore;
use spin_key_value_sqlite::{DatabaseLocation, KeyValueSqlite};

/// A key-value store that uses SQLite as the backend.
pub struct SpinKeyValueStore {
    /// The base path or directory for the SQLite database file.
    base_path: Option<PathBuf>,
}

impl SpinKeyValueStore {
    /// Create a new SpinKeyValueStore with the given base path.
    ///
    /// If `base_path` is `Some`, the database will be stored at the combined
    /// `base_path` and the `path` specified in the runtime configuration. Otherwise,
    /// only if the `path` in the runtime config is an absolute path will it be used as is.
    /// In all other cases, an in-memory database will be used.
    pub fn new(base_path: Option<PathBuf>) -> Self {
        Self { base_path }
    }
}

impl MakeKeyValueStore for SpinKeyValueStore {
    const RUNTIME_CONFIG_TYPE: &'static str = "spin";

    type RuntimeConfig = SpinKeyValueRuntimeConfig;

    type StoreManager = KeyValueSqlite;

    fn make_store(
        &self,
        runtime_config: Self::RuntimeConfig,
    ) -> anyhow::Result<Self::StoreManager> {
        let location = match (&self.base_path, &runtime_config.path) {
            // If both the base path and the path are specified, resolve the path against the base path
            (Some(base_path), Some(path)) => {
                let path = resolve_relative_path(path, base_path);
                DatabaseLocation::Path(path)
            }
            // If the base path is `None` but path is an absolute path, use the absolute path
            (None, Some(path)) if path.is_absolute() => DatabaseLocation::Path(path.clone()),
            // Otherwise, use an in-memory database
            _ => DatabaseLocation::InMemory,
        };
        if let DatabaseLocation::Path(path) = &location {
            // Create the store's parent directory if necessary
            if let Some(parent) = path.parent().filter(|p| !p.exists()) {
                fs::create_dir_all(parent)
                    .context("Failed to create key value store's parent directory")?;
            }
        }
        Ok(KeyValueSqlite::new(location))
    }
}

/// The serialized runtime configuration for the SQLite key-value store.
#[derive(Deserialize, Serialize)]
pub struct SpinKeyValueRuntimeConfig {
    /// The path to the SQLite database file.
    path: Option<PathBuf>,
}

impl SpinKeyValueRuntimeConfig {
    /// The default filename for the SQLite database.
    const DEFAULT_SPIN_STORE_FILENAME: &'static str = "sqlite_key_value.db";
}

impl Default for SpinKeyValueRuntimeConfig {
    fn default() -> Self {
        Self {
            path: Some(PathBuf::from(Self::DEFAULT_SPIN_STORE_FILENAME)),
        }
    }
}

/// Resolve a relative path against a base dir.
///
/// If the path is absolute, it is returned as is. Otherwise, it is resolved against the base dir.
fn resolve_relative_path(path: &Path, base_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_owned();
    }
    base_dir.join(path)
}
