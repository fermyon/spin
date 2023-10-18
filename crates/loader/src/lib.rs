//! Loaders for Spin applications.
//! This crate implements the possible application sources for Spin applications,
//! and includes functionality to convert the specific configuration (for example
//! local configuration files or from OCI) into Spin configuration that
//! can be consumed by the Spin execution context.
//!
//! This crate can be extended (or replaced entirely) to support additional loaders,
//! and any implementation that produces a `Application` is compatible
//! with the Spin execution context.

#![deny(missing_docs)]

use std::path::{Path, PathBuf};

use anyhow::Result;
use local::LocalLoader;
use spin_common::paths::parent_dir;
use spin_locked_app::locked::LockedApp;

pub mod cache;
mod http;
mod local;

/// Maximum number of files to copy (or download) concurrently
pub(crate) const MAX_FILE_LOADING_CONCURRENCY: usize = 16;

/// Load a Spin locked app from a spin.toml manifest file. If `files_mount_root`
/// is given, `files` mounts will be copied to that directory. If not, `files`
/// mounts will validated as "direct mounts".
pub async fn from_file(
    manifest_path: impl AsRef<Path>,
    files_mount_strategy: FilesMountStrategy,
) -> Result<LockedApp> {
    let path = manifest_path.as_ref();
    let app_root = parent_dir(path)?;
    let loader = LocalLoader::new(&app_root, files_mount_strategy).await?;
    loader.load_file(path).await
}

/// The strategy to use for mounting WASI files into a guest.
#[derive(Debug)]
pub enum FilesMountStrategy {
    /// Copy files into the given mount root directory.
    Copy(PathBuf),
    /// Mount files directly from their source director(ies). This only
    /// supports mounting full directories; mounting single files, glob
    /// patterns, and `exclude_files` are not supported.
    Direct,
}
