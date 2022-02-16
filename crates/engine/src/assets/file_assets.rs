#![deny(missing_docs)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::log;

use super::*;

#[rustfmt::skip]
pub(crate) async fn prepare(
    file_patterns: &[String],
    source_base_directory: impl AsRef<Path>,
    destination_base_directory: impl AsRef<Path>,
    component_id: &str,
) -> Result<DirectoryMount> {
    log::info!(
        "Mounting files from '{}' to '{}'",
        source_base_directory.as_ref().display(),
        destination_base_directory.as_ref().display()
    );

    let files_to_mount =
        collect_all(file_patterns, source_base_directory)?;
    let asset_directory =
        create_asset_directory(&destination_base_directory, component_id).await?;
    copy_all(&files_to_mount, &asset_directory).await?;

    Ok(DirectoryMount {
        host: asset_directory,
        guest: "/".to_string(),
    })
}

struct FileMount {
    source_path: PathBuf,
    destination_relative_path: String,
}

impl FileMount {
    pub fn from_path(
        source_path: Result<PathBuf, glob::GlobError>,
        relative_to: impl AsRef<Path>,
    ) -> Result<Self> {
        let source_path = source_path?;
        let destination_relative_path = to_relative(&source_path, &relative_to)?;
        Ok(Self {
            source_path,
            destination_relative_path,
        })
    }
}

fn collect_all(
    file_patterns: &[String],
    relative_to: impl AsRef<Path>,
) -> Result<Vec<FileMount>> {
    let results = file_patterns.iter().map(|pattern| {
        collect_pattern(pattern, &relative_to)
            .with_context(|| format!("Failed to collect file mounts for {}", pattern))
    });
    let collections = results.into_iter().collect::<Result<Vec<_>>>()?;
    let collection = collections.into_iter().flatten().collect();
    Ok(collection)
}

fn collect_pattern(
    pattern: &str,
    relative_to: impl AsRef<Path>,
) -> Result<Vec<FileMount>> {
    let absolute_pattern_buf = relative_to.as_ref().join(pattern); // Without the let binding we get a 'dropped while borrowed' error
    let absolute_pattern = absolute_pattern_buf.to_string_lossy();
    log::debug!("Resolving asset file pattern '{}'", absolute_pattern);

    let matches = glob::glob(&absolute_pattern)?;
    let specifiers = matches
        .into_iter()
        .map(|path| FileMount::from_path(path, &relative_to))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let files: Vec<_> = specifiers
        .into_iter()
        .filter(|s| s.source_path.is_file())
        .collect();
    ensure_all_under(&relative_to, files.iter().map(|s| &s.source_path))?;
    Ok(files)
}

#[rustfmt::skip]
async fn copy_all(
    files: &[FileMount],
    asset_directory: impl AsRef<Path>,
) -> Result<()> {
    let futures = files
        .iter()
        .map(|f| copy_one(f, &asset_directory));
    let results = futures::future::join_all(futures).await;
    let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
    for e in &errors {
        log::error!("{:#}", e);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Error copying assets: {} file(s) not copied", errors.len()))
    }
}

#[rustfmt::skip]
async fn copy_one(
    file: &FileMount,
    asset_directory: impl AsRef<Path>,
) -> Result<()> {
    let from = &file.source_path;
    let to = asset_directory.as_ref().join(&file.destination_relative_path);

    ensure_under(&asset_directory, &to)?;

    log::trace!("Copying asset file '{}' -> '{}'", from.display(), to.display());
    tokio::fs::create_dir_all(to.parent().expect("Cannot copy to file '/'")).await?;
    
    tokio::fs::copy(&from, &to)
        .await
        .with_context(|| format!("Error copying asset file  '{}'", from.display()))?;
    
    Ok(())
}
