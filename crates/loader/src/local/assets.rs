#![deny(missing_docs)]

use crate::assets::{create_dir, ensure_all_under, ensure_under, to_relative};
use anyhow::{anyhow, bail, Context, Result};
use futures::future;
use spin_config::DirectoryMount;
use std::path::{Path, PathBuf};
use tracing::log;
use walkdir::WalkDir;

use super::config::{RawDirectoryPlacement, RawFileMount};

/// Prepare all local assets given a component ID and its file patterns.
/// This file will copy all assets into a temporary directory as read-only.
pub(crate) async fn prepare_component(
    raw_mounts: &[RawFileMount],
    src: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
    id: &str,
) -> Result<Vec<DirectoryMount>> {
    log::info!(
        "Mounting files from '{}' to '{}'",
        src.as_ref().display(),
        base_dst.as_ref().display()
    );

    let files = collect(raw_mounts, src)?;
    let host = create_dir(&base_dst, id).await?;
    let guest = "/".to_string();
    copy_all(&files, &host).await?;

    Ok(vec![DirectoryMount { guest, host }])
}

/// A file that a component requires to be present at runtime.
#[derive(Debug, Clone)]
pub struct FileMount {
    /// The source
    pub src: PathBuf,
    /// The location where the component expects the file.
    pub relative_dst: String,
}

impl FileMount {
    fn from(
        src: Result<impl AsRef<Path>, glob::GlobError>,
        relative_to: impl AsRef<Path>,
    ) -> Result<Self> {
        let src = src?;
        let relative_dst = to_relative(&src, &relative_to)?;
        let src = src.as_ref().to_path_buf();
        Ok(Self { src, relative_dst })
    }

    fn from_exact(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<Self> {
        let src = src.as_ref().to_path_buf();
        let relative_dst = dest.as_ref().to_string_lossy().to_string();
        Ok(Self { src, relative_dst })
    }
}

/// Generate a vector of file mounts for a component given all its file patterns.
pub fn collect(raw_mounts: &[RawFileMount], rel: impl AsRef<Path>) -> Result<Vec<FileMount>> {
    let (patterns, placements) = uncase(raw_mounts);

    let pattern_files = collect_patterns(&patterns, &rel)?;
    let placement_files = collect_placements(&placements, &rel)?;
    let all_files = [pattern_files, placement_files].concat();
    Ok(all_files)
}

fn collect_placements(
    placements: &[RawDirectoryPlacement],
    rel: impl AsRef<Path>,
) -> Result<Vec<FileMount>, anyhow::Error> {
    let results = placements.iter().map(|placement| {
        collect_placement(placement, &rel).with_context(|| {
            format!(
                "Failed to collect file mounts for {}",
                placement.source.display()
            )
        })
    });
    let collections = results.collect::<Result<Vec<_>>>()?;
    let collection = collections.into_iter().flatten().collect();
    Ok(collection)
}

fn collect_patterns(
    patterns: &[String],
    rel: impl AsRef<Path>,
) -> Result<Vec<FileMount>, anyhow::Error> {
    let results = patterns.iter().map(|pattern| {
        collect_pattern(pattern, &rel)
            .with_context(|| format!("Failed to collect file mounts for {}", pattern))
    });
    let collections = results.collect::<Result<Vec<_>>>()?;
    let collection = collections.into_iter().flatten().collect();
    Ok(collection)
}

fn collect_placement(
    placement: &RawDirectoryPlacement,
    rel: impl AsRef<Path>,
) -> Result<Vec<FileMount>> {
    let source = &placement.source;
    let guest_path = &placement.destination;

    if !source.is_relative() {
        bail!(
            "Cannot place {}: source paths must be relative",
            source.display()
        );
    }
    // TODO: check if this works if the host is Windows
    if !guest_path.is_absolute() {
        bail!(
            "Cannot place {}: guest paths must be absolute",
            guest_path.display()
        );
    }
    // TODO: okay to assume that absolute guest paths start with '/'?
    let relative_guest_path = guest_path.strip_prefix("/")?;

    let abs = rel.as_ref().join(source);
    if !abs.is_dir() {
        bail!("Cannot place {}: source must be a directory", abs.display());
    }

    let walker = WalkDir::new(&abs);
    let files = walker
        .into_iter()
        .filter_map(|de| match de {
            Err(e) => Some(
                Err(e).with_context(|| format!("Failed to walk directory under {}", abs.display())),
            ),
            Ok(dir_entry) => {
                if dir_entry.file_type().is_file() {
                    let match_path = dir_entry.path();
                    match to_relative(match_path, &abs) {
                        Ok(relative_to_match_root_dst) => {
                            let guest_dst = relative_guest_path.join(relative_to_match_root_dst);
                            Some(FileMount::from_exact(match_path, &guest_dst))
                        }
                        Err(e) => {
                            let err = Err(e).with_context(|| {
                                format!(
                                    "Failed to establish relative path for '{}'",
                                    match_path.display()
                                )
                            });
                            Some(err)
                        }
                    }
                } else {
                    None
                }
            }
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(files)
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

    let perms = tokio::fs::metadata(&to).await?.permissions();
    // perms.set_readonly(true);
    tokio::fs::set_permissions(&to, perms).await?;

    Ok(())
}

fn uncase(raw_mounts: &[RawFileMount]) -> (Vec<String>, Vec<RawDirectoryPlacement>) {
    (
        raw_mounts.iter().filter_map(as_pattern).collect(),
        raw_mounts.iter().filter_map(as_placement).collect(),
    )
}

fn as_pattern(fm: &RawFileMount) -> Option<String> {
    match fm {
        RawFileMount::Pattern(p) => Some(p.to_owned()),
        _ => None,
    }
}
fn as_placement(fm: &RawFileMount) -> Option<RawDirectoryPlacement> {
    match fm {
        RawFileMount::Placement(p) => Some(p.clone()),
        _ => None,
    }
}
