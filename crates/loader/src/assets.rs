#![deny(missing_docs)]

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Create the temporary directory for a component.
pub(crate) async fn create_dir(base: impl AsRef<Path>, id: &str) -> Result<PathBuf> {
    let dir = base.as_ref().join("assets").join(component_dir(id));

    fs::create_dir_all(&dir)
        .await
        .with_context(|| anyhow!("Error creating temporary asset directory {}", dir.display()))?;

    Ok(dir)
}

/// Get the path of a file relative to a given directory.
pub fn to_relative(path: impl AsRef<Path>, relative_to: impl AsRef<Path>) -> Result<String> {
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
        .replace('\\', "/"))
}

/// Ensure all paths are under a given directory.
pub(crate) fn ensure_all_under(
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
pub(crate) fn ensure_under(desired: impl AsRef<Path>, actual: impl AsRef<Path>) -> Result<()> {
    match is_under(&desired, &actual) {
        true => Ok(()),
        false => bail!(
            "Error copying assets: copy to '{}' outside the application directory",
            actual.as_ref().display()
        ),
    }
}

// Check whether a path is under a given directory.
pub(crate) fn is_under(desired: impl AsRef<Path>, actual: impl AsRef<Path>) -> bool {
    // TODO: There should be a more robust check here.
    actual.as_ref().strip_prefix(desired.as_ref()).is_ok()
        && !(actual.as_ref().display().to_string().contains(".."))
}

lazy_static::lazy_static! {
    static ref UNSAFE_CHARACTERS: regex::Regex = regex::Regex::new("[^-_a-zA-Z0-9]").expect("Invalid identifier regex");
}

/// Generate a directory for a component using the (sanitized) component ID and its SHA256.
pub(crate) fn component_dir(id: &str) -> String {
    // Using the SHA could generate quite long directory names, which could be a problem on Windows
    // if the asset paths are also long. Longer term, consider an alternative approach where
    // we use an index or something for disambiguation, and/or disambiguating only if a clash is
    // detected, etc.
    let id_sha256 = crate::digest::bytes_sha256_string(id.as_bytes());
    format!("{}_{}", UNSAFE_CHARACTERS.replace_all(id, "_"), id_sha256)
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
