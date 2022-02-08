#![deny(missing_docs)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::Digest;
use tracing::log;

// Prefer this to a tuple because it's not clear which way round the tuple is
// (guest->host or host->guest)
#[derive(Clone, Debug)]
pub(crate) struct DirectoryMount {
    pub guest: String,
    pub host: PathBuf,
}

#[rustfmt::skip]
pub(crate) async fn prepare_local_assets(
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
        collect_file_mounts(file_patterns, source_base_directory)?;
    let component_directory =
        create_mount_directory(&destination_base_directory, component_id).await?;
    copy_all_to(&files_to_mount, &component_directory).await?;

    Ok(DirectoryMount {
        host: component_directory,
        guest: "/".to_string(),
    })
}

struct FileMountSpecifier {
    source_path: PathBuf,
    destination_relative_path: String,
}

impl FileMountSpecifier {
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

fn collect_file_mounts(
    file_patterns: &[String],
    relative_to: impl AsRef<Path>,
) -> Result<Vec<FileMountSpecifier>> {
    let results = file_patterns.iter().map(|pattern| {
        collect_one_pattern(pattern, &relative_to)
            .with_context(|| format!("Failed to collect file mounts for {}", pattern))
    });
    let collections = results.into_iter().collect::<Result<Vec<_>>>()?;
    let collection = collections.into_iter().flatten().collect();
    Ok(collection)
}

fn collect_one_pattern(
    pattern: &str,
    relative_to: impl AsRef<Path>,
) -> Result<Vec<FileMountSpecifier>> {
    let absolute_pattern_buf = relative_to.as_ref().join(pattern); // Without the let binding we get a 'dropped while borrowed' error
    let absolute_pattern = absolute_pattern_buf.to_string_lossy();
    log::debug!("Resolving asset file pattern '{}'", absolute_pattern);

    let matches = glob::glob(&absolute_pattern)?;
    let specifiers = matches
        .into_iter()
        .map(|path| FileMountSpecifier::from_path(path, &relative_to))
        .collect::<anyhow::Result<Vec<_>>>()?;
    ensure_all_under(&relative_to, specifiers.iter().map(|s| &s.source_path))?;
    Ok(specifiers)
}

#[rustfmt::skip]
async fn create_mount_directory(
    base_directory: impl AsRef<Path>,
    component_id: &str,
) -> Result<PathBuf> {
    let component_directory = component_dir(component_id);
    let mount_directory = base_directory
        .as_ref()
        .join("assets")
        .join(component_directory);

    tokio::fs::create_dir_all(&mount_directory)
        .await
        .with_context(|| {
            format!("Error creating temporary asset directory {}", mount_directory.display())
        })?;

    Ok(mount_directory)
}

lazy_static::lazy_static! {
    static ref UNSAFE_CHARACTERS: regex::Regex = regex::Regex::new("[^-_a-zA-Z0-9]").expect("Invalid identifier regex");
}

fn component_dir(component_id: &str) -> String {
    // Using the SHA could generate quite long directory names, which could be a problem on Windows
    // if the asset paths are also long. Longer term, consider an alternative approach where
    // we use an index or something for disambiguation, and/or disambiguating only if a clash is
    // detected, etc.
    let safe_name = UNSAFE_CHARACTERS.replace_all(component_id, "_");
    let disambiguator = digest_string(component_id);
    format!("{}_{}", safe_name, disambiguator)
}

#[rustfmt::skip]
async fn copy_all_to(
    files_to_mount: &[FileMountSpecifier],
    mount_directory: impl AsRef<Path>,
) -> Result<()> {
    let futures = files_to_mount
        .iter()
        .map(|f| copy_one_to(f, &mount_directory));
    let results = futures::future::join_all(futures).await;
    let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
    for e in &errors {
        log::error!("{}", e);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Error copying assets: {} file(s) not copied", errors.len()))
    }
}

#[rustfmt::skip]
async fn copy_one_to(
    file_to_mount: &FileMountSpecifier,
    mount_directory: impl AsRef<Path>,
) -> Result<()> {
    let from = &file_to_mount.source_path;
    let to = mount_directory.as_ref().join(&file_to_mount.destination_relative_path);

    ensure_under(&mount_directory, &to)?;

    log::trace!("Copying asset file '{}' -> '{}'", from.display(), to.display());
    tokio::fs::create_dir_all(to.parent().expect("Cannot copy to file '/'")).await?;
    let _ = tokio::fs::copy(&from, &to)
        .await
        .with_context(|| format!("Error copying asset file  '{}'", from.display()))?;
    Ok(())
}

#[rustfmt::skip]
fn to_relative(path: impl AsRef<Path>, relative_to: impl AsRef<Path>) -> Result<String> {
    let relative_path = path.as_ref().strip_prefix(&relative_to).with_context(|| {
        format!("Copied path '{}' did not belong with expected prefix '{}'", path.as_ref().display(), relative_to.as_ref().display())
    })?;

    let relative_path_string = relative_path
        .to_str()
        .ok_or_else(|| {
            anyhow::anyhow!("Can't convert '{}' back to relative path", relative_path.display())
        })?
        .to_owned()
        .replace("\\", "/"); // TODO: a better way

    Ok(relative_path_string)
}

#[rustfmt::skip]
fn ensure_all_under<T: AsRef<Path>>(
    desired_path: impl AsRef<Path>,
    actual_paths: impl Iterator<Item = T>,
) -> Result<()> {
    let not_under: Vec<_> = actual_paths
        .filter(|p| !is_under(&desired_path, &p.as_ref()))
        .collect();

    if not_under.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Error copying assets: {} file(s) were outside the application directory", not_under.len()))
    }
}

#[rustfmt::skip]
fn ensure_under(desired_path: impl AsRef<Path>, actual_path: impl AsRef<Path>) -> Result<()> {
    if is_under(&desired_path, &actual_path) {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Error copying assets: copy to '{}' would be outside the application directory", actual_path.as_ref().display()))
    }
}

#[rustfmt::skip]
fn is_under(desired_path: impl AsRef<Path>, actual_path: impl AsRef<Path>) -> bool {
    // TODO: This is a tragic kludge and I'm sure ingenious people could still find a way
    // to fool it. But there doesn't seem to be an actual reliable solution!
    actual_path.as_ref().strip_prefix(desired_path.as_ref()).is_ok()
        && !(actual_path.as_ref().display().to_string().contains(".."))
}

fn digest_string(text: &str) -> String {
    let mut sha = sha2::Sha256::new();
    sha.update(text.as_bytes());
    let digest_value = sha.finalize();
    format!("{:x}", digest_value)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_under() {
        assert_eq!(true, is_under("/foo", "/foo/bar"));
        assert_eq!(false, is_under("/foo", "/bar/baz"));
        assert_eq!(false, is_under("/foo", "/foo/../bar/baz"));
    }
}
