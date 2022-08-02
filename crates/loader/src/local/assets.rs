#![deny(missing_docs)]

use crate::assets::{
    change_file_permission, create_dir, ensure_all_under, ensure_under, to_relative,
};
use anyhow::{anyhow, bail, ensure, Context, Result};
use futures::{future, stream, StreamExt};
use spin_manifest::DirectoryMount;
use std::{
    path::{Path, PathBuf},
    vec,
};
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
    allow_transient_write: bool,
    exclude_files: &[String],
) -> Result<Vec<DirectoryMount>> {
    log::info!(
        "Mounting files from '{}' to '{}'",
        src.as_ref().display(),
        base_dst.as_ref().display()
    );

    let files = collect(raw_mounts, exclude_files, src)?;
    let host = create_dir(&base_dst, id).await?;
    let guest = "/".to_string();
    copy_all(&files, &host, allow_transient_write).await?;

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
    fn from(src: impl AsRef<Path>, relative_to: impl AsRef<Path>) -> Result<Self> {
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
pub fn collect(
    raw_mounts: &[RawFileMount],
    exclude_files: &[String],
    rel: impl AsRef<Path>,
) -> Result<Vec<FileMount>> {
    let (patterns, placements) = uncase(raw_mounts);

    let pattern_files = collect_patterns(&patterns, &rel)?;
    let placement_files = collect_placements(&placements, &rel)?;
    let all_files = [pattern_files, placement_files].concat();

    let exclude_patterns = convert_strings_to_glob_patterns(exclude_files, &rel)?;
    Ok(get_included_files(all_files, &exclude_patterns))
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
    if !is_absolute_guest_path(guest_path) {
        bail!(
            "Cannot place at {}: guest paths must be absolute",
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
    log::trace!("Resolving asset file pattern '{:?}'", abs);

    let matches = glob::glob(&abs.to_string_lossy())?;
    let specifiers = matches
        .into_iter()
        .map(|path| FileMount::from(path?, &rel))
        .collect::<Result<Vec<_>>>()?;
    let files: Vec<_> = specifiers.into_iter().filter(|s| s.src.is_file()).collect();
    ensure_all_under(&rel, files.iter().map(|s| &s.src))?;
    Ok(files)
}

/// Copy all files to the mount directory.
async fn copy_all(
    files: &[FileMount],
    dir: impl AsRef<Path>,
    allow_transient_write: bool,
) -> Result<()> {
    let copy_futures = files.iter().map(|f| copy(f, &dir, allow_transient_write));
    let errors = stream::iter(copy_futures)
        .buffer_unordered(crate::MAX_PARALLEL_ASSET_PROCESSING)
        .filter_map(|r| future::ready(r.err()))
        .map(|e| log::error!("{:?}", e))
        .count()
        .await;
    ensure!(
        errors == 0,
        "Error copying assets: {} file(s) not copied",
        errors
    );
    Ok(())
}

/// Copy a single file to the mount directory, setting it as read-only.
async fn copy(file: &FileMount, dir: impl AsRef<Path>, allow_transient_write: bool) -> Result<()> {
    let from = &file.src;
    let to = dir.as_ref().join(&file.relative_dst);

    ensure_under(&dir.as_ref(), &to.as_path())?;

    log::trace!(
        "Copying asset file '{}' -> '{}'",
        from.display(),
        to.display()
    );

    tokio::fs::create_dir_all(to.parent().expect("Cannot copy to file '/'")).await?;

    // if destination file is read-only, set it to writable first
    let metadata = tokio::fs::metadata(&to).await;
    if metadata.is_ok() && metadata.unwrap().permissions().readonly() {
        change_file_permission(&to, true).await?;
    }

    let _ = tokio::fs::copy(&from, &to)
        .await
        .with_context(|| anyhow!("Error copying asset file  '{}'", from.display()))?;

    change_file_permission(&to, allow_transient_write).await?;

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

fn is_absolute_guest_path(path: impl AsRef<Path>) -> bool {
    // We can't use `is_absolute` to check that guest paths are absolute,
    // because that would use the logic of the host filesystem.  If the
    // host is Windows, that would mean a path like `/assets` would not
    // be considered absolute, and a path like `e:\assets` would be. But
    // the Wasmtime preopened directory interface only works - as far as I
    // can tell - with Unix-style guest paths. So we have to check these
    // paths specifically using Unix logic rather than the system function.
    path.as_ref().to_string_lossy().starts_with('/')
}

/// Convert strings to glob patterns
fn convert_strings_to_glob_patterns<T: AsRef<str>>(
    files: &[T],
    rel: impl AsRef<Path>,
) -> Result<Vec<glob::Pattern>> {
    let file_paths = files
        .iter()
        .map(|f| match rel.as_ref().join(f.as_ref()).to_str() {
            Some(abs) => Ok(abs.to_owned()),
            None => Err(anyhow!(
                "Can't join {} and {}",
                rel.as_ref().display(),
                f.as_ref()
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    file_paths
        .iter()
        .map(|f| {
            glob::Pattern::new(f).with_context(|| format!("can't convert {} to glob pattern", f))
        })
        .collect::<Result<Vec<glob::Pattern>>>()
}

/// Remove files which match excluded patterns
fn get_included_files(files: Vec<FileMount>, exclude_patterns: &[glob::Pattern]) -> Vec<FileMount> {
    files
        .into_iter()
        .filter(|f| {
            for exclude_pattern in exclude_patterns {
                if exclude_pattern.matches_path(Path::new(&f.src)) {
                    tracing::info!(
                        "file: {} is excluded by pattern {}",
                        f.src.display(),
                        exclude_pattern
                    );
                    return false;
                }
            }
            true
        })
        .collect::<Vec<_>>()
}
