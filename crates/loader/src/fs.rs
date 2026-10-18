use anyhow::{Context, Result};
use std::path::Path;

/// Creates a temporary file in the directory of `path`, to be persisted over it.
/// On Unix the file is created with mode 0666 less the umask, as `std::fs::write`
/// would create it, rather than tempfile's default of 0600.
fn temp_file_beside(path: &Path) -> Result<tempfile::NamedTempFile> {
    let dir = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o666));
    }
    builder
        .tempfile_in(dir)
        .with_context(|| format!("cannot create temporary file in {}", dir.display()))
}

#[cfg(feature = "async-io")]
mod io {
    use super::*;

    /// Writes `bytes` to a temporary file that is then renamed over `path`, so a
    /// concurrent reader never sees a partially written file.
    pub async fn write_file_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
        let tmp = temp_file_beside(path)?;
        tokio::fs::write(tmp.path(), bytes).await?;
        tmp.persist(path)?;
        Ok(())
    }

    pub async fn create_dir_all(path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(path).await?;
        Ok(())
    }

    pub async fn copy(from: &Path, to: &Path) -> Result<u64> {
        tokio::fs::copy(from, to).await.map_err(Into::into)
    }

    pub async fn metadata(path: &Path) -> Result<std::fs::Metadata> {
        tokio::fs::metadata(path).await.map_err(Into::into)
    }
}

#[cfg(not(feature = "async-io"))]
mod io {
    use super::*;

    /// Writes `bytes` to a temporary file that is then renamed over `path`, so a
    /// concurrent reader never sees a partially written file.
    pub async fn write_file_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
        let tmp = temp_file_beside(path)?;
        std::fs::write(tmp.path(), bytes)?;
        tmp.persist(path)?;
        Ok(())
    }

    pub async fn create_dir_all(path: &Path) -> Result<()> {
        std::fs::create_dir_all(path)?;
        Ok(())
    }

    pub async fn copy(from: &Path, to: &Path) -> Result<u64> {
        Ok(std::fs::copy(from, to)?)
    }

    pub async fn metadata(path: &Path) -> Result<std::fs::Metadata> {
        Ok(std::fs::metadata(path)?)
    }
}

pub use io::*;
