use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use spin_common::data_dir::data_dir;
use std::{
    ffi::OsStr,
    fs::{self, File},
    path::{Path, PathBuf},
};
use tar::Archive;

use crate::{error::*, manifest::PluginManifest};

/// Directory where the manifests of installed plugins are stored.
pub const PLUGIN_MANIFESTS_DIRECTORY_NAME: &str = "manifests";
const INSTALLATION_RECORD_FILE_NAME: &str = ".install.json";

/// Houses utilities for getting the path to Spin plugin directories.
pub struct PluginStore {
    root: PathBuf,
}

impl PluginStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn try_default() -> Result<Self> {
        Ok(Self::new(data_dir()?.join("plugins")))
    }

    /// Gets the path to where Spin plugin are installed.
    pub fn get_plugins_directory(&self) -> &Path {
        &self.root
    }

    /// Get the path to the subdirectory of an installed plugin.
    pub fn plugin_subdirectory_path(&self, plugin_name: &str) -> PathBuf {
        self.root.join(plugin_name)
    }

    /// Get the path to the manifests directory which contains the plugin manifests
    /// of all installed Spin plugins.
    pub fn installed_manifests_directory(&self) -> PathBuf {
        self.root.join(PLUGIN_MANIFESTS_DIRECTORY_NAME)
    }

    pub fn installed_manifest_path(&self, plugin_name: &str) -> PathBuf {
        self.installed_manifests_directory()
            .join(manifest_file_name(plugin_name))
    }

    pub fn installed_binary_path(&self, plugin_name: &str) -> PathBuf {
        let mut binary = self.root.join(plugin_name).join(plugin_name);
        if cfg!(target_os = "windows") {
            binary.set_extension("exe");
        }
        binary
    }

    pub fn installation_record_file(&self, plugin_name: &str) -> PathBuf {
        self.root
            .join(plugin_name)
            .join(INSTALLATION_RECORD_FILE_NAME)
    }

    pub fn installed_manifests(&self) -> Result<Vec<PluginManifest>> {
        let manifests_dir = self.installed_manifests_directory();
        let manifest_paths = Self::json_files_in(&manifests_dir);
        let manifests = manifest_paths
            .iter()
            .filter_map(|path| Self::try_read_manifest_from(path))
            .collect();
        Ok(manifests)
    }

    // TODO: report errors on individuals
    pub fn catalogue_manifests(&self) -> Result<Vec<PluginManifest>> {
        // Structure:
        // CATALOGUE_DIR (spin/plugins/.spin-plugins/manifests)
        // |- foo
        // |  |- foo@0.1.2.json
        // |  |- foo@1.2.3.json
        // |  |- foo.json
        // |- bar
        //    |- bar.json
        let catalogue_dir =
            crate::lookup::spin_plugins_repo_manifest_dir(self.get_plugins_directory());

        // Catalogue directory doesn't exist so likely nothing has been installed.
        if !catalogue_dir.exists() {
            return Ok(Vec::new());
        }

        let plugin_dirs = catalogue_dir
            .read_dir()
            .context("reading manifest catalogue at {catalogue_dir:?}")?
            .filter_map(|d| d.ok())
            .map(|d| d.path())
            .filter(|p| p.is_dir());
        let manifest_paths = plugin_dirs.flat_map(|path| Self::json_files_in(&path));
        let manifests: Vec<_> = manifest_paths
            .filter_map(|path| Self::try_read_manifest_from(&path))
            .collect();
        Ok(manifests)
    }

    fn try_read_manifest_from(manifest_path: &Path) -> Option<PluginManifest> {
        let manifest_file = File::open(manifest_path).ok()?;
        serde_json::from_reader(manifest_file).ok()
    }

    fn json_files_in(dir: &Path) -> Vec<PathBuf> {
        let json_ext = Some(OsStr::new("json"));
        match dir.read_dir() {
            Err(_) => vec![],
            Ok(rd) => rd
                .filter_map(|de| de.ok())
                .map(|de| de.path())
                .filter(|p| p.is_file() && p.extension() == json_ext)
                .collect(),
        }
    }

    /// Returns the PluginManifest for an installed plugin with a given name.
    /// Looks up and parses the JSON plugin manifest file into object form.
    pub fn read_plugin_manifest(&self, plugin_name: &str) -> PluginLookupResult<PluginManifest> {
        let manifest_path = self.installed_manifest_path(plugin_name);
        tracing::info!("Reading plugin manifest from {}", manifest_path.display());
        let manifest_file = File::open(manifest_path.clone()).map_err(|e| {
            Error::NotFound(NotFoundError::new(
                Some(plugin_name.to_string()),
                manifest_path.display().to_string(),
                e.to_string(),
            ))
        })?;
        let manifest = serde_json::from_reader(manifest_file).map_err(|e| {
            Error::InvalidManifest(InvalidManifestError::new(
                Some(plugin_name.to_string()),
                manifest_path.display().to_string(),
                e.to_string(),
            ))
        })?;
        Ok(manifest)
    }

    pub(crate) fn add_manifest(&self, plugin_manifest: &PluginManifest) -> Result<()> {
        let manifests_dir = self.installed_manifests_directory();
        std::fs::create_dir_all(manifests_dir)?;
        serde_json::to_writer(
            &File::create(self.installed_manifest_path(&plugin_manifest.name()))?,
            plugin_manifest,
        )?;
        tracing::trace!("Added manifest for {}", &plugin_manifest.name());
        Ok(())
    }

    pub(crate) fn untar_plugin(&self, plugin_file_name: &PathBuf, plugin_name: &str) -> Result<()> {
        // Get handle to file
        let tar_gz = File::open(plugin_file_name)?;
        // Unzip file
        let tar = GzDecoder::new(tar_gz);
        // Get plugin from tarball
        let mut archive = Archive::new(tar);
        archive.set_preserve_permissions(true);
        // Create subdirectory in plugins directory for this plugin
        let plugin_sub_dir = self.plugin_subdirectory_path(plugin_name);
        fs::remove_dir_all(&plugin_sub_dir).ok();
        fs::create_dir_all(&plugin_sub_dir)?;
        archive.unpack(&plugin_sub_dir)?;
        Ok(())
    }
}

/// Given a plugin name, returns the expected file name for the installed manifest
pub fn manifest_file_name(plugin_name: &str) -> String {
    format!("{plugin_name}.json")
}
