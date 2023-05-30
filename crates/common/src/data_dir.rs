//! Resolves Spin's default data directory paths

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Return the default data directory for Spin
pub fn default_data_dir() -> Result<PathBuf> {
    if let Some(pkg_mgr_dir) = package_manager_data_dir() {
        return Ok(pkg_mgr_dir);
    }

    let data_dir = dirs::data_local_dir()
        .or_else(|| dirs::home_dir().map(|p| p.join(".spin")))
        .ok_or_else(|| anyhow!("Unable to get local data directory or home directory"))?;
    Ok(data_dir.join("spin"))
}

/// Get the package manager specific data directory
fn package_manager_data_dir() -> Option<PathBuf> {
    if let Ok(brew_prefix) = std::env::var("HOMEBREW_PREFIX") {
        let data_dir = Path::new(&brew_prefix).join("var").join("spin");

        if data_dir.is_dir() {
            return Some(data_dir);
            // TODO: check if they also have plugins in non-brew default dir and warn
        }
    }
    None
}
