use anyhow::Result;
use std::path::Path;

#[cfg(feature = "async-io")]
mod io {
    use super::*;

    pub async fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
        tokio::fs::write(path, bytes).await?;
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

    pub async fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
        std::fs::write(path, bytes)?;
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
