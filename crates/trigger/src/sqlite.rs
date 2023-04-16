use std::path::Path;

use crate::runtime_config::RuntimeConfig;
use anyhow::Context;
use spin_sqlite::{DatabaseLocation, SqliteComponent};

// TODO: dedup with the stuff in key_value
pub(crate) fn build_component(runtime_config: &RuntimeConfig) -> anyhow::Result<SqliteComponent> {
    let location = match runtime_config.sqlite_db_path() {
        Some(path) => {
            // Create the store's parent directory if necessary
            create_parent_dir(&path).context("Failed to create sqlite db")?;
            DatabaseLocation::Path(path)
        }
        None => DatabaseLocation::InMemory,
    };

    Ok(SqliteComponent::new(location))
}

fn create_parent_dir(path: &Path) -> anyhow::Result<()> {
    let dir = path
        .parent()
        .with_context(|| format!("{path:?} missing parent dir"))?;
    std::fs::create_dir_all(dir).with_context(|| format!("Failed to create parent dir {dir:?}"))
}
