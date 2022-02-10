#![deny(missing_docs)]

mod file_assets;
mod bindle_assets;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::Digest;
use spin_config::ReferencedFiles;

pub(crate) async fn prepare(
    component_id: &str,
    assets: &ReferencedFiles,
    destination_base_directory: impl AsRef<std::path::Path>,
) -> Result<Vec<DirectoryMount>> {
    match assets {
        ReferencedFiles::None =>
            Ok(vec![]),
        ReferencedFiles::FilePatterns(source_directory, file_patterns) => {
            let mount = file_assets::prepare(
                file_patterns,
                source_directory,
                &destination_base_directory,
                component_id,
            )
            .await
            .with_context(|| {
                format!("Error copying assets for component '{}'", component_id)
            })?;
            Ok(vec![mount])
        },
        ReferencedFiles::BindleParcels(reader, invoice_id, parcels) => {
            let mount = bindle_assets::prepare(
                &reader,
                &invoice_id,
                &parcels,
                &destination_base_directory,
                component_id,
            )
            .await
            .with_context(|| {
                format!("Error copying assets for component '{}'", component_id)
            })?;
            Ok(vec![mount])
        },
    }
}

// Prefer this to a tuple because it's not clear which way round the tuple is
// (guest->host or host->guest)
#[derive(Clone, Debug)]
pub(crate) struct DirectoryMount {
    pub guest: String,
    pub host: PathBuf,
}

#[rustfmt::skip]
async fn create_asset_directory(
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
        assert!(is_under("/foo", "/foo/bar"));
        assert!(!is_under("/foo", "/bar/baz"));
        assert!(!is_under("/foo", "/foo/../bar/baz"));
    }
}
