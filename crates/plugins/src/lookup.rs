use crate::{error::*, git::GitSource, manifest::PluginManifest, store::manifest_file_name};
use semver::Version;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use tracing::log;
use url::Url;

// Name of directory that contains the cloned centralized Spin plugins
// repository
const PLUGINS_REPO_LOCAL_DIRECTORY: &str = ".spin-plugins";

// Name of directory containing the installed manifests
const PLUGINS_REPO_MANIFESTS_DIRECTORY: &str = "manifests";

const SPIN_PLUGINS_REPO: &str = "https://github.com/fermyon/spin-plugins/";

/// Looks up plugin manifests in centralized spin plugin repository.
pub struct PluginLookup {
    pub name: String,
    pub version: Option<Version>,
}

impl PluginLookup {
    pub fn new(name: &str, version: Option<Version>) -> Self {
        Self {
            name: name.to_lowercase(),
            version,
        }
    }

    pub async fn get_manifest_from_repository(
        &self,
        plugins_dir: &Path,
    ) -> PluginLookupResult<PluginManifest> {
        let url = plugins_repo_url()?;
        log::info!("Pulling manifest for plugin {} from {url}", self.name);
        fetch_plugins_repo(&url, plugins_dir, false)
            .await
            .map_err(|e| {
                Error::ConnectionFailed(ConnectionFailedError::new(url.to_string(), e.to_string()))
            })?;
        let expected_path = spin_plugins_repo_manifest_path(&self.name, &self.version, plugins_dir);
        let file = File::open(&expected_path).map_err(|e| {
            Error::NotFound(NotFoundError::new(
                Some(self.name.clone()),
                expected_path.display().to_string(),
                e.to_string(),
            ))
        })?;
        let manifest: PluginManifest = serde_json::from_reader(file).map_err(|e| {
            Error::InvalidManifest(InvalidManifestError::new(
                Some(self.name.clone()),
                expected_path.display().to_string(),
                e.to_string(),
            ))
        })?;
        Ok(manifest)
    }
}

pub fn plugins_repo_url() -> Result<Url, url::ParseError> {
    Url::parse(SPIN_PLUGINS_REPO)
}

pub async fn fetch_plugins_repo(
    repo_url: &Url,
    plugins_dir: &Path,
    update: bool,
) -> anyhow::Result<()> {
    let git_root = plugin_manifests_repo_path(plugins_dir);
    let git_source = GitSource::new(repo_url, None, &git_root);
    if git_root.join(".git").exists() {
        if update {
            git_source.pull().await?;
        }
    } else {
        git_source.clone_repo().await?;
    }
    Ok(())
}

fn plugin_manifests_repo_path(plugins_dir: &Path) -> PathBuf {
    plugins_dir.join(PLUGINS_REPO_LOCAL_DIRECTORY)
}

// Given a name and option version, outputs expected file name for the plugin.
fn manifest_file_name_version(plugin_name: &str, version: &Option<semver::Version>) -> String {
    match version {
        Some(v) => format!("{}@{}.json", plugin_name, v),
        None => manifest_file_name(plugin_name),
    }
}

/// Get expected path to the manifest of a plugin with a given name
/// and version within the spin-plugins repository
fn spin_plugins_repo_manifest_path(
    plugin_name: &str,
    plugin_version: &Option<Version>,
    plugins_dir: &Path,
) -> PathBuf {
    spin_plugins_repo_manifest_dir(plugins_dir)
        .join(plugin_name)
        .join(manifest_file_name_version(plugin_name, plugin_version))
}

pub fn spin_plugins_repo_manifest_dir(plugins_dir: &Path) -> PathBuf {
    plugins_dir
        .join(PLUGINS_REPO_LOCAL_DIRECTORY)
        .join(PLUGINS_REPO_MANIFESTS_DIRECTORY)
}
