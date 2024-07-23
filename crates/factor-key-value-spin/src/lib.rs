use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use spin_factor_key_value::MakeKeyValueStore;
use spin_key_value_sqlite::{DatabaseLocation, KeyValueSqlite};

/// A key-value store that uses SQLite as the backend.
pub struct SpinKeyValueStore {
    /// The base path or directory for the SQLite database file.
    base_path: PathBuf,
}

impl SpinKeyValueStore {
    /// Create a new SpinKeyValueStore with the given base path.
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
}

/// Runtime configuration for the SQLite key-value store.
#[derive(Deserialize, Serialize)]
pub struct SpinKeyValueRuntimeConfig {
    /// The path to the SQLite database file.
    path: Option<PathBuf>,
}

impl SpinKeyValueRuntimeConfig {
    /// The default filename for the SQLite database.
    const DEFAULT_SPIN_STORE_FILENAME: &'static str = "sqlite_key_value.db";

    /// Create a new runtime configuration with the given state directory.
    ///
    /// If the database directory is None, the database is in-memory.
    /// If the database directory is Some, the database is stored in a file in the state directory.
    pub fn default(default_database_dir: Option<PathBuf>) -> Self {
        let path = default_database_dir.map(|dir| dir.join(Self::DEFAULT_SPIN_STORE_FILENAME));
        Self { path }
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

impl MakeKeyValueStore for SpinKeyValueStore {
    const RUNTIME_CONFIG_TYPE: &'static str = "spin";

    type RuntimeConfig = SpinKeyValueRuntimeConfig;

    type StoreManager = KeyValueSqlite;

    fn make_store(
        &self,
        runtime_config: Self::RuntimeConfig,
    ) -> anyhow::Result<Self::StoreManager> {
        let location = match runtime_config.path {
            Some(path) => {
                let path = resolve_relative_path(&path, &self.base_path);
                // Create the store's parent directory if necessary
                fs::create_dir_all(path.parent().unwrap())
                    .context("Failed to create key value store")?;
                DatabaseLocation::Path(path)
            }
            None => DatabaseLocation::InMemory,
        };
        Ok(KeyValueSqlite::new(location))
    }
}
