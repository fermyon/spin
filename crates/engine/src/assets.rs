#![deny(missing_docs)]

use anyhow::{anyhow, bail, Context, Result};
use futures::future;
use sha2::Digest;
use std::path::{Path, PathBuf};
use tracing::log;

/// Directory mount for the assets of a component.
#[derive(Clone, Debug)]
pub(crate) struct DirectoryMount {
    /// Guest directory destination for mounting inside the module.
    pub(crate) guest: String,
    /// Host directory source for mounting inside the module.
    pub(crate) host: PathBuf,
}

/// Prepare all local assets given a component ID and its file patterns.
/// This file will copy all assets into a temporary directory as read-only.
pub(crate) async fn prepare(
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

/// Create the temporary directory for a component.
async fn create_dir(base: impl AsRef<Path>, id: &str) -> Result<PathBuf> {
    let dir = base.as_ref().join("assets").join(component_dir(id));

    tokio::fs::create_dir_all(&dir)
        .await
        .with_context(|| anyhow!("Error creating temporary asset directory {}", dir.display()))?;

    Ok(dir)
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

/// Get the path of a file relative to a given directory.
fn to_relative(path: impl AsRef<Path>, relative_to: impl AsRef<Path>) -> Result<String> {
    let rel = path.as_ref().strip_prefix(&relative_to).with_context(|| {
        format!(
            "Copied path '{}' did not belong with expected prefix '{}'",
            path.as_ref().display(),
            relative_to.as_ref().display()
        )
    })?;

    Ok(rel
        .to_str()
        .ok_or_else(|| anyhow!("Can't convert '{}' back to relative path", rel.display()))?
        .to_owned()
        // TODO: a better way
        .replace("\\", "/"))
}

/// Ensure all paths are under a given directory.
fn ensure_all_under(
    desired: impl AsRef<Path>,
    paths: impl Iterator<Item = impl AsRef<Path>>,
) -> Result<()> {
    match paths.filter(|p| !is_under(&desired, p.as_ref())).count() {
        0 => Ok(()),
        n => bail!(
            "Error copying assets: {} file(s) were outside the application directory",
            n
        ),
    }
}

/// Return an error if a path is not under a given directory.
fn ensure_under(desired: impl AsRef<Path>, actual: impl AsRef<Path>) -> Result<()> {
    match is_under(&desired, &actual) {
        true => Ok(()),
        false => bail!(
            "Error copying assets: copy to '{}' outside the application directory",
            actual.as_ref().display()
        ),
    }
}

// Check whether a path is under a given directory.
fn is_under(desired: impl AsRef<Path>, actual: impl AsRef<Path>) -> bool {
    // TODO: There should be a more robust check here.
    actual.as_ref().strip_prefix(desired.as_ref()).is_ok()
        && !(actual.as_ref().display().to_string().contains(".."))
}

lazy_static::lazy_static! {
    static ref UNSAFE_CHARACTERS: regex::Regex = regex::Regex::new("[^-_a-zA-Z0-9]").expect("Invalid identifier regex");
}

/// Generate a directory for a component using the (sanitized) component ID and its SHA256.
fn component_dir(id: &str) -> String {
    // Using the SHA could generate quite long directory names, which could be a problem on Windows
    // if the asset paths are also long. Longer term, consider an alternative approach where
    // we use an index or something for disambiguation, and/or disambiguating only if a clash is
    // detected, etc.
    format!("{}_{}", UNSAFE_CHARACTERS.replace_all(id, "_"), sha256(id))
}

/// Return the SHA256 digest of the input text.
fn sha256(text: &str) -> String {
    let mut sha = sha2::Sha256::new();
    sha.update(text.as_bytes());
    format!("{:x}", sha.finalize())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_under() {
        assert!(is_under("/foo", "/foo/bar"));
        assert!(!is_under("/foo", "/bar/baz"));
        assert!(!is_under("/foo", "/foo/../bar/baz"));
    }
}
