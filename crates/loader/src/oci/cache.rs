//! Cache for OCI registry entities.

use anyhow::{bail, Context, Result};
use oci_distribution::Reference;
use tokio::fs;

use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = "spin";
const REGISTRY_CACHE_DIR: &str = "registry";
const MANIFESTS_DIR: &str = "manifests";
const WASM_DIR: &str = "wasm";
const DATA_DIR: &str = "data";

const MANIFEST_FILE: &str = "manifest.json";
const CONFIG_FILE: &str = "config.json";
const LATEST_TAG: &str = "latest";

/// Cache for registry entities.
pub struct Cache {
    /// Root directory for the cache instance.
    pub root: PathBuf,
}

impl Cache {
    /// Create a new cache given an optional root directory.
    pub async fn new(root: Option<PathBuf>) -> Result<Self> {
        let root = match root {
            Some(root) => root,
            None => dirs::cache_dir()
                .context("cannot get cache directory")?
                .join(CONFIG_DIR),
        };
        let root = root.join(REGISTRY_CACHE_DIR);
        Self::ensure_dirs(&root).await?;

        Ok(Self { root })
    }

    /// The manifests directory for the current cache.
    fn manifests_dir(&self) -> PathBuf {
        self.root.join(MANIFESTS_DIR)
    }

    /// The Wasm bytes directory for the current cache.
    fn wasm_dir(&self) -> PathBuf {
        self.root.join(WASM_DIR)
    }

    /// The data directory for the current cache.
    fn data_dir(&self) -> PathBuf {
        self.root.join(DATA_DIR)
    }

    /// Return the path to a wasm file given its digest.
    pub fn wasm_file(&self, digest: impl AsRef<str>) -> Result<PathBuf> {
        let path = &self.wasm_dir().join(digest.as_ref());
        match path.exists() {
            true => Ok(path.into()),
            false => bail!(format!(
                "cannot find wasm file for digest {}",
                digest.as_ref()
            )),
        }
    }

    /// Return the path to a data file given its digest.
    pub fn data_file(&self, digest: impl AsRef<str>) -> Result<PathBuf> {
        let path = &self.data_dir().join(digest.as_ref());
        match path.exists() {
            true => Ok(path.into()),
            false => bail!(format!(
                "cannot find data file for digest {}",
                digest.as_ref()
            )),
        }
    }

    /// Get the file path to an OCI manifest given a reference.
    /// If the directory for the manifest does not exist, this will create it.
    pub async fn oci_manifest_path(&self, reference: impl AsRef<str>) -> Result<PathBuf> {
        let reference: Reference = reference
            .as_ref()
            .parse()
            .context("cannot parse OCI reference")?;
        let p = self
            .manifests_dir()
            .join(reference.registry())
            .join(reference.repository())
            .join(reference.tag().unwrap_or(LATEST_TAG));

        if !p.is_dir() {
            fs::create_dir_all(&p)
                .await
                .context("cannot find directory for OCI manifest")?;
        }

        Ok(p.join(MANIFEST_FILE))
    }

    /// Get the file path to the OCI configuration object given a reference.
    pub async fn lockfile_path(&self, reference: impl AsRef<str>) -> Result<PathBuf> {
        let reference: Reference = reference
            .as_ref()
            .parse()
            .context("cannot parse reference")?;
        let p = self
            .manifests_dir()
            .join(reference.registry())
            .join(reference.repository())
            .join(reference.tag().unwrap_or(LATEST_TAG));

        if !p.is_dir() {
            fs::create_dir_all(&p)
                .await
                .context("cannot find configuration object for reference")?;
        }

        Ok(p.join(CONFIG_FILE))
    }

    /// Write the contents in the cache's wasm directory.
    pub async fn write_wasm(&self, bytes: impl AsRef<[u8]>, digest: impl AsRef<str>) -> Result<()> {
        fs::write(self.wasm_dir().join(digest.as_ref()), bytes.as_ref()).await?;
        Ok(())
    }

    /// Write the contents in the cache's data directory.
    pub async fn write_data(&self, bytes: impl AsRef<[u8]>, digest: impl AsRef<str>) -> Result<()> {
        fs::write(self.data_dir().join(digest.as_ref()), bytes.as_ref()).await?;
        Ok(())
    }

    /// Ensure the expected configuration directories are found in the root.
    /// └── <configuration-root>
    ///     └── registry
    ///             └──manifests
    ///             └──wasm
    ///             └─-data
    async fn ensure_dirs(root: &Path) -> Result<()> {
        tracing::debug!("using cache root directory {}", root.display());

        let p = root.join(MANIFESTS_DIR);
        if !p.is_dir() {
            fs::create_dir_all(&p).await.with_context(|| {
                format!("failed to create manifests directory `{}`", p.display())
            })?;
        }

        let p = root.join(WASM_DIR);
        if !p.is_dir() {
            fs::create_dir_all(&p)
                .await
                .with_context(|| format!("failed to create wasm directory `{}`", p.display()))?;
        }

        let p = root.join(DATA_DIR);
        if !p.is_dir() {
            fs::create_dir_all(&p)
                .await
                .with_context(|| format!("failed to create assets directory `{}`", p.display()))?;
        }

        Ok(())
    }
}
