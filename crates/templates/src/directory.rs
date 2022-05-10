use std::path::{Path, PathBuf};

use anyhow::Context;

pub(crate) fn subdirectories(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let dir_entries = std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read contents of '{}' directory", dir.display()))?;
    let directories = dir_entries
        .filter_map(as_dir_path)
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| {
            format!(
                "Failed to find subdirectories of '{}' directory",
                dir.display()
            )
        })?;
    Ok(directories)
}

fn as_dir_path(dir_entry: std::io::Result<std::fs::DirEntry>) -> Option<std::io::Result<PathBuf>> {
    match dir_entry {
        Err(e) => Some(Err(e)),
        Ok(entry) => match entry.file_type() {
            Err(e) => Some(Err(e)),
            Ok(ty) => {
                if ty.is_dir() {
                    Some(Ok(entry.path()))
                } else {
                    None
                }
            }
        },
    }
}
