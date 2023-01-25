use std::{
    fmt::{Debug, Display, Error, Formatter},
    path::{PathBuf, Path},
    str::FromStr,
};

use anyhow::Result;
use tempfile::tempdir;

#[derive(Clone, Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(path: PathBuf) -> TempDir {
        Self { path }
    }
    pub fn as_path(&self) -> &Path {
        self.path.as_path()
    }
}

impl Default for TempDir {
    fn default() -> Self {
        Self::new(tempdir().expect("Temp").path().to_path_buf())
    }
}

impl Display for TempDir {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        Ok(self.path.fmt(f)?)
    }
}

impl FromStr for TempDir {
    type Err = <PathBuf as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(PathBuf::from_str(s)?))
    }
}
