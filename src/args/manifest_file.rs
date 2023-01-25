use std::{
    fmt,
    fmt::{Debug, Display, Formatter},
    path::{Path, PathBuf},
    str::FromStr, convert::Infallible,
};

pub use anyhow::{Result, Error};
pub use crate::opts::DEFAULT_MANIFEST_FILE;

#[derive(Clone, Debug)]
pub struct ManifestFile {
    relative_path: PathBuf
}

impl ManifestFile {
    pub fn new(relative_path: PathBuf) -> Self {
        Self { relative_path }
    }
    pub fn canonicalize(&self) -> Result<PathBuf> {
        Ok(self.relative_path.canonicalize()?)
    }

    pub async fn build(&self) -> Result<()> {
        spin_build::build(&self.relative_path).await
    }
}

impl AsRef<Path> for ManifestFile {
    fn as_ref(&self) -> &Path {
        &self.relative_path
    }
}

impl From<&Path> for ManifestFile {
    fn from(value: &Path) -> Self {
        value.to_path_buf().into()
    }
}

impl From<PathBuf> for ManifestFile {
    fn from(value: PathBuf) -> Self {
        Self::new(value)
    }
}

impl Default for ManifestFile {
    fn default() -> Self {
        PathBuf::from(DEFAULT_MANIFEST_FILE).into()
    }
}

impl FromStr for ManifestFile {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Infallible> {
        PathBuf::from_str(s).map(Self::from)
    }
}

impl Display for ManifestFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        self.relative_path.fmt(f)
    }
}
