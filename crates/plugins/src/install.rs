use crate::{
    get_manifest_file_name, get_manifest_file_name_version,
    version_check::{assert_supported_version, get_plugin_manifest},
    PLUGIN_MANIFESTS_DIRECTORY_NAME,
};

use super::git::GitSource;
use super::plugin_manifest::{Os, PluginManifest};
use super::prompt::Prompter;
use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use semver::Version;
use std::{
    fs::{self, File},
    io::{copy, Cursor},
    path::{Path, PathBuf},
};
use tar::Archive;
use tempfile::{tempdir, TempDir};
use url::Url;

/// Name of the subdirectory that contains the installed plugin JSON manifests
const PLUGINS_REPO_LOCAL_DIRECTORY: &str = ".spin-plugins";
const PLUGINS_REPO_MANIFESTS_DIRECTORY: &str = "manifests";

pub enum ManifestLocation {
    Local(PathBuf),
    Remote(Url),
    PluginsRepository(PluginInfo),
}

pub struct PluginInfo {
    name: String,
    repo_url: Url,
    version: Option<Version>,
}
impl PluginInfo {
    pub fn new(name: String, repo_url: Url, version: Option<Version>) -> Self {
        Self {
            name,
            repo_url,
            version,
        }
    }
}

pub struct PluginInstaller {
    manifest_location: ManifestLocation,
    plugins_dir: PathBuf,
    yes_to_all: bool,
    spin_version: String,
}

impl PluginInstaller {
    pub fn new(
        manifest_location: ManifestLocation,
        plugins_dir: PathBuf,
        yes_to_all: bool,
        spin_version: &str,
    ) -> Self {
        Self {
            manifest_location,
            plugins_dir,
            yes_to_all,
            spin_version: spin_version.to_string(),
        }
    }

    pub async fn install(&self) -> Result<()> {
        // TODO: Potentially handle errors to give useful error messages
        let plugin_manifest: PluginManifest = match &self.manifest_location {
            ManifestLocation::Remote(url) => {
                // Remote manifest source is provided
                log::info!("Pulling manifest for plugin from {}", url);
                reqwest::get(url.as_ref())
                    .await?
                    .json::<PluginManifest>()
                    .await?
            }
            ManifestLocation::Local(path) => {
                // Local manifest source is provided
                log::info!("Pulling manifest for plugin from {:?}", path);
                let file = File::open(path)?;
                serde_json::from_reader(file)?
            }
            ManifestLocation::PluginsRepository(info) => {
                log::info!(
                    "Pulling manifest for plugin {} from {}",
                    info.name,
                    info.repo_url
                );
                let git_source = GitSource::new(
                    &info.repo_url,
                    None,
                    self.plugins_dir.join(PLUGINS_REPO_LOCAL_DIRECTORY),
                )?;
                if !self
                    .plugins_dir
                    .join(PLUGINS_REPO_LOCAL_DIRECTORY)
                    .join(".git")
                    .exists()
                {
                    git_source.clone().await?;
                } else {
                    git_source.pull().await?;
                }
                let file = File::open(
                    &self
                        .plugins_dir
                        .join(PLUGINS_REPO_LOCAL_DIRECTORY)
                        .join(PLUGINS_REPO_MANIFESTS_DIRECTORY)
                        .join(&info.name)
                        .join(get_manifest_file_name_version(&info.name, &info.version)),
                )
                .map_err(|_| {
                    anyhow!(
                        "Could not find plugin [{} {:?}] in centralized repository",
                        info.name,
                        info.version
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| String::from("latest"))
                    )
                })?;
                serde_json::from_reader(file)?
            }
        };

        // Return early if plugin is already installed with latest version
        if let Ok(installed) = get_plugin_manifest(&plugin_manifest.name, &self.plugins_dir) {
            if installed.version >= plugin_manifest.version {
                return Err(anyhow!(
                    "plugin {} already installed with version {} but attempting to install same or older version ({})",
                    installed.name,
                    installed.version,
                    plugin_manifest.version,
                ));
            }
        }

        assert_supported_version(&self.spin_version, &plugin_manifest.spin_compatibility)?;

        let os: Os = if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else if cfg!(target_os = "macos") {
            Os::Osx
        } else {
            return Err(anyhow!("This plugin is not supported on this OS"));
        };
        // TODO: Add logic for architecture as well
        let plugin_package = plugin_manifest
            .packages
            .iter()
            .find(|p| p.os == os)
            .ok_or_else(|| anyhow!("This plugin does not support this OS"))?;
        let target_url = plugin_package.url.to_owned();

        // Ask for user confirmation if not overridden with CLI option
        if !self.yes_to_all
            && !Prompter::new(&plugin_manifest.name, &plugin_manifest.license, &target_url)?
                .run()?
        {
            // User has requested to not install package, returning early
            println!("Plugin {} will not be installed", plugin_manifest.name);
            return Ok(());
        }
        let temp_dir = tempdir()?;
        let plugin_file_name =
            PluginInstaller::download_plugin(&plugin_manifest.name, &temp_dir, &target_url).await?;
        self.verify_checksum(&plugin_file_name, &plugin_package.sha256)?;

        self.untar_plugin(&plugin_file_name, &plugin_manifest.name)?;
        // Save manifest to installed plugins directory
        self.add_to_manifest_dir(&plugin_manifest)?;
        log::info!("Plugin installed successfully");
        Ok(())
    }

    fn untar_plugin(&self, plugin_file_name: &PathBuf, plugin_name: &str) -> Result<()> {
        // Get handle to file
        let tar_gz = File::open(&plugin_file_name)?;
        // Unzip file
        let tar = GzDecoder::new(tar_gz);
        // Get plugin from tarball
        let mut archive = Archive::new(tar);
        // TODO: this is unix only. Look into whether permissions are preserved
        archive.set_preserve_permissions(true);
        // Create subdirectory in plugins directory for this plugin
        let plugin_sub_dir = self.plugins_dir.join(plugin_name);
        fs::create_dir_all(&plugin_sub_dir)?;
        archive.unpack(&plugin_sub_dir)?;
        Ok(())
    }

    async fn download_plugin(name: &str, temp_dir: &TempDir, target_url: &str) -> Result<PathBuf> {
        log::info!(
            "Trying to get tar file for plugin {} from {}",
            name,
            target_url
        );
        let plugin_bin = reqwest::get(target_url).await?;
        let mut content = Cursor::new(plugin_bin.bytes().await?);
        let dir = temp_dir.path();
        let mut plugin_file = dir.join(name);
        plugin_file.set_extension("tar.gz");
        let mut temp_file = File::create(&plugin_file)?;
        copy(&mut content, &mut temp_file)?;
        Ok(plugin_file)
    }

    // Validate checksum of downloaded content with checksum from Index
    fn verify_checksum(&self, plugin_file: &PathBuf, checksum: &str) -> Result<()> {
        let binary_sha256 = file_digest_string(plugin_file).expect("failed to get sha for parcel");
        let verification_sha256 = checksum;
        if binary_sha256 == verification_sha256 {
            println!("Package verified successfully");
            Ok(())
        } else {
            Err(anyhow!("Could not validate Checksum"))
        }
    }

    fn add_to_manifest_dir(&self, plugin: &PluginManifest) -> Result<()> {
        let manifests_dir = self.plugins_dir.join(PLUGIN_MANIFESTS_DIRECTORY_NAME);
        fs::create_dir_all(&manifests_dir)?;
        serde_json::to_writer(
            &File::create(manifests_dir.join(get_manifest_file_name(&plugin.name)))?,
            plugin,
        )?;
        Ok(())
    }
}

fn file_digest_string(path: impl AsRef<Path>) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(&path)?;
    let mut sha = Sha256::new();
    std::io::copy(&mut file, &mut sha)?;
    let digest_value = sha.finalize();
    let digest_string = format!("{:x}", digest_value);
    Ok(digest_string)
}
