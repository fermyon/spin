use crate::{git::GitSource, manifest::PluginManifest, store::manifest_file_name};
use anyhow::{Context, Result};
use semver::Version;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use url::Url;

// Name of directory that contains the cloned centralized Spin plugins
// repository
const PLUGINS_REPO_LOCAL_DIRECTORY: &str = ".spin-plugins";

// Name of directory containing the installed manifests
const PLUGINS_REPO_MANIFESTS_DIRECTORY: &str = "manifests";

pub const PLUGIN_NOT_FOUND_ERROR_MSG: &str = "plugin not found";

/// Looks up plugin manifests in centralized spin plugin repository.
pub struct PluginLookup {
    pub name: String,
    repo_url: Url,
    pub version: Option<Version>,
}

impl PluginLookup {
    pub fn new(name: &str, repo_url: Url, version: Option<Version>) -> Self {
        Self {
            name: name.to_lowercase(),
            repo_url,
            version,
        }
    }

    fn version_string(&self) -> String {
        self.version
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| String::from("latest"))
    }

    pub async fn get_manifest_from_repository(&self, plugins_dir: &Path) -> Result<PluginManifest> {
        log::info!(
            "Pulling manifest for plugin {} from {}",
            self.name,
            self.repo_url
        );
        let git_root = plugin_manifests_repo_path(plugins_dir);
        let git_source = GitSource::new(&self.repo_url, None, &git_root)?;
        if !git_root.join(".git").exists() {
            git_source.clone().await?;
        } else {
            // TODO: consider moving this to a separate `spin plugin
            // update` subcommand rather than always updating the
            // repository on each install.
            git_source.pull().await?;
        }
        let file = File::open(spin_plugins_repo_manifest_path(
            &self.name,
            &self.version,
            plugins_dir,
        ))
        .with_context(|| {
            format!(
                "{} {} {} in centralized repository",
                self.name,
                self.version_string(),
                PLUGIN_NOT_FOUND_ERROR_MSG,
            )
        })?;
        let manifest: PluginManifest = serde_json::from_reader(file)?;
        Ok(manifest)
    }
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
    plugins_dir
        .join(PLUGINS_REPO_LOCAL_DIRECTORY)
        .join(PLUGINS_REPO_MANIFESTS_DIRECTORY)
        .join(plugin_name)
        .join(manifest_file_name_version(plugin_name, plugin_version))
}
