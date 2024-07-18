use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use spin_factor_key_value::MakeKeyValueStore;
use spin_key_value_sqlite::{DatabaseLocation, KeyValueSqlite};

pub struct SpinKeyValueStore {
    base_path: PathBuf,
}

impl SpinKeyValueStore {
    pub fn new(base_path: Option<PathBuf>) -> anyhow::Result<Self> {
        let base_path = match base_path {
            Some(path) => path,
            None => std::env::current_dir().context("failed to get current directory")?,
        };
        Ok(Self { base_path })
    }
}

#[derive(Deserialize, Serialize)]
pub struct SpinKeyValueRuntimeConfig {
    path: Option<PathBuf>,
}

impl SpinKeyValueRuntimeConfig {
    const DEFAULT_SPIN_STORE_FILENAME: &'static str = "sqlite_key_value.db";

    pub fn default(state_dir: Option<PathBuf>) -> Self {
        let path = state_dir.map(|dir| dir.join(Self::DEFAULT_SPIN_STORE_FILENAME));
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
