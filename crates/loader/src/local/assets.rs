#![deny(missing_docs)]

use crate::assets::{create_dir, ensure_all_under, ensure_under, to_relative};
use anyhow::{anyhow, bail, Context, Result};
use futures::future;
use spin_config::DirectoryMount;
use std::path::{Path, PathBuf};
use tracing::log;

/// Prepare all local assets given a component ID and its file patterns.
/// This file will copy all assets into a temporary directory as read-only.
pub(crate) async fn prepare_component(
    patterns: &[String],
    src: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
    id: &str,
) -> Result<DirectoryMount> {
    log::info!(
        "Mounting files from '{}' to '{}'",
        src.as_ref().display(),
        base_dst.as_ref().display()
    );

    let files = collect(patterns, src)?;
    let host = create_dir(&base_dst, id).await?;
    let guest = "/".to_string();
    copy_all(&files, &host).await?;

    Ok(DirectoryMount { guest, host })
}

struct FileMount {
    src: PathBuf,
    relative_dst: String,
}

impl FileMount {
    fn from(
        src: Result<impl AsRef<Path>, glob::GlobError>,
        relative_dst: impl AsRef<Path>,
    ) -> Result<Self> {
        let src = src?;
        let relative_dst = to_relative(&src, &relative_dst)?;
        let src = src.as_ref().to_path_buf();
        Ok(Self { src, relative_dst })
    }
}

/// Generate a vector of file mounts for a component given all its file patterns.
fn collect(patterns: &[String], rel: impl AsRef<Path>) -> Result<Vec<FileMount>> {
    Ok(patterns
        .iter()
        .map(|pattern| {
            collect_pattern(pattern, &rel)
                .with_context(|| anyhow!("Failed to collect file mounts for {}", pattern))
        })
        .flatten()
        .flatten()
        .collect())
}

/// Generate a vector of file mounts given a file pattern.
fn collect_pattern(pattern: &str, rel: impl AsRef<Path>) -> Result<Vec<FileMount>> {
    let abs = rel.as_ref().join(pattern);
    log::debug!("Resolving asset file pattern '{:?}'", abs);

    let matches = glob::glob(&abs.to_string_lossy())?;
    let specifiers = matches
        .into_iter()
        .map(|path| FileMount::from(path, &rel))
        .collect::<Result<Vec<_>>>()?;
    let files: Vec<_> = specifiers.into_iter().filter(|s| s.src.is_file()).collect();
    ensure_all_under(&rel, files.iter().map(|s| &s.src))?;
    Ok(files)
}

/// Copy all files to the mount directory.
async fn copy_all(files: &[FileMount], dir: impl AsRef<Path>) -> Result<()> {
    let res = future::join_all(files.iter().map(|f| copy(f, &dir))).await;
    match res
        .into_iter()
        .filter_map(|r| r.err())
        .map(|e| log::error!("{:?}", e))
        .count()
    {
        0 => Ok(()),
        n => bail!("Error copying assets: {} file(s) not copied", n),
    }
}

/// Copy a single file to the mount directory, setting it as read-only.
async fn copy(file: &FileMount, dir: impl AsRef<Path>) -> Result<()> {
    let from = &file.src;
    let to = dir.as_ref().join(&file.relative_dst);

    ensure_under(&dir.as_ref(), &to.as_path())?;

    log::trace!(
        "Copying asset file '{}' -> '{}'",
        from.display(),
        to.display()
    );

    tokio::fs::create_dir_all(to.parent().expect("Cannot copy to file '/'")).await?;

    let _ = tokio::fs::copy(&from, &to)
        .await
        .with_context(|| anyhow!("Error copying asset file  '{}'", from.display()))?;

    let mut perms = tokio::fs::metadata(&to).await?.permissions();
    perms.set_readonly(true);
    tokio::fs::set_permissions(&to, perms).await?;

    Ok(())
}
