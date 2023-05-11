use anyhow::anyhow;
use std::path::{Path, PathBuf};

use crate::opts::DEFAULT_MANIFEST_FILE;

/// Resolves a manifest path provided by a user, which may be a file or
/// directory, to a path to a manifest file.
pub(crate) fn resolve_file_path(provided_path: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    let path = provided_path.as_ref();

    if path.is_file() {
        Ok(path.to_owned())
    } else if path.is_dir() {
        let file_path = path.join(DEFAULT_MANIFEST_FILE);
        if file_path.is_file() {
            Ok(file_path)
        } else {
            Err(anyhow!(
                "Directory {} does not contain a file named 'spin.toml'",
                path.display()
            ))
        }
    } else {
        let pd = path.display();
        let err = match path.try_exists() {
            Err(e) => anyhow!("Error accessing path {pd}: {e:#}"),
            Ok(false) => anyhow!("No such file or directory '{pd}'"),
            Ok(true) => anyhow!("Path {pd} is neither a file nor a directory"),
        };
        Err(err)
    }
}
