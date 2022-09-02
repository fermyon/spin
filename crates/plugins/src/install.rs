use crate::{
    get_manifest_file_name, get_manifest_file_name_version,
    version_check::{assert_supported_version, get_plugin_manifest},
    PLUGIN_MANIFESTS_DIRECTORY_NAME, SPIN_INTERNAL_COMMANDS,
};

use super::git::GitSource;
use super::plugin_manifest::{Os, PluginManifest};
use super::prompt::Prompter;
use anyhow::{anyhow, bail, Result};
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

// Name of directory that contains the cloned centralized Spin plugins
// repository
const PLUGINS_REPO_LOCAL_DIRECTORY: &str = ".spin-plugins";
// Name of directory containing the installed manifests
const PLUGINS_REPO_MANIFESTS_DIRECTORY: &str = "manifests";
// Url scheme prefix of a plugin that is installed from a local source
const URL_FILE_SCHEME: &str = "file";

/// Location of manifest of the plugin to be installed.
pub enum ManifestLocation {
    /// Plugin manifest can be copied from a local path.
    Local(PathBuf),
    /// Plugin manifest should be pulled from a specific address.
    Remote(Url),
    /// Plugin manifest lives in the centralized plugins repository
    PluginsRepository(PluginInfo),
}

/// Information about the plugin manifest that should be fetched from the
/// centralized Spin plugins repository.
pub struct PluginInfo {
    name: String,
    repo_url: Url,
    version: Option<Version>,
}

impl PluginInfo {
    pub fn new(name: &str, repo_url: Url, version: Option<Version>) -> Self {
        Self {
            name: name.to_string(),
            repo_url,
            version,
        }
    }
}

/// Retrieves the appropriate plugin manifest and installs the Spin plugin
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

    /// Installs a Spin plugin. First attempts to retrieve the plugin manifest.
    /// If installing a plugin from the centralized Spin plugins repository, it
    /// fetches the latest contents of the repository and searches for the
    /// appropriately named and versioned plugin manifest. Parses the plugin
    /// manifest to get the appropriate source for the machine OS and
    /// architecture. Verifies the checksum of the source, unpacks and installs
    /// it into the plugins directory.
    pub async fn install(&self) -> Result<()> {
        let plugin_manifest: PluginManifest = match &self.manifest_location {
            ManifestLocation::Remote(url) => {
                log::info!("Pulling manifest for plugin from {}", url);
                reqwest::get(url.as_ref())
                    .await?
                    .json::<PluginManifest>()
                    .await?
            }
            ManifestLocation::Local(path) => {
                log::info!("Pulling manifest for plugin from {:?}", path);
                let file = File::open(path)
                    .map_err(|_| anyhow!("The local manifest could not be opened"))?;
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
                    // TODO: consider moving this to a separate `spin plugin
                    // update` subcommand rather than always updating the
                    // repository on each install.
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

        // Disallow installing plugins with the same name as spin internal
        // subcommands
        if SPIN_INTERNAL_COMMANDS
            .iter()
            .any(|&s| s == plugin_manifest.name())
        {
            bail!(
                "Trying to install a plugin with the same name '{}' as an internal plugin",
                plugin_manifest.name()
            );
        }

        // Disallow downgrades and reinstalling identical plugins
        if let Ok(installed) = get_plugin_manifest(&plugin_manifest.name(), &self.plugins_dir) {
            if installed.version > plugin_manifest.version || installed == plugin_manifest {
                bail!(
                    "plugin {} already installed with version {} but attempting to install same or older version ({})",
                    installed.name(),
                    installed.version,
                    plugin_manifest.version,
                );
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
            bail!("This plugin is not supported on this OS");
        };
        let arch = std::env::consts::ARCH;
        let plugin_package = plugin_manifest
            .packages
            .iter()
            .find(|p| p.os == os && p.arch.to_string() == arch)
            .ok_or_else(|| anyhow!("This plugin does not support this OS or architecture"))?;
        let target = plugin_package.url.to_owned();

        // Ask for user confirmation to install if not overridden with CLI
        // option
        if !self.yes_to_all
            && !Prompter::new(&plugin_manifest.name(), &plugin_manifest.license, &target)?.run()?
        {
            // User has requested to not install package, returning early
            println!("Plugin {} will not be installed", plugin_manifest.name());
            return Ok(());
        }
        let target_url = Url::parse(&target)?;
        let temp_dir = tempdir()?;
        let plugin_tarball_path = match target_url.scheme() {
            URL_FILE_SCHEME => PathBuf::from(target_url.path()),
            _ => {
                PluginInstaller::download_plugin(&plugin_manifest.name(), &temp_dir, &target)
                    .await?
            }
        };
        self.verify_checksum(&plugin_tarball_path, &plugin_package.sha256)?;

        self.untar_plugin(&plugin_tarball_path, &plugin_manifest.name())?;

        // Save manifest to installed plugins directory
        self.add_to_manifest_dir(&plugin_manifest)?;
        println!(
            "Plugin [{}] was installed successfully!",
            plugin_manifest.name()
        );
        Ok(())
    }

    fn untar_plugin(&self, plugin_file_name: &PathBuf, plugin_name: &str) -> Result<()> {
        // Get handle to file
        let tar_gz = File::open(&plugin_file_name)?;
        // Unzip file
        let tar = GzDecoder::new(tar_gz);
        // Get plugin from tarball
        let mut archive = Archive::new(tar);
        archive.set_preserve_permissions(true);
        // Create subdirectory in plugins directory for this plugin
        let plugin_sub_dir = self.plugins_dir.join(plugin_name);
        fs::remove_dir_all(&plugin_sub_dir).ok();
        fs::create_dir_all(&plugin_sub_dir)?;
        archive.unpack(&plugin_sub_dir)?;
        Ok(())
    }

    async fn download_plugin(name: &str, temp_dir: &TempDir, target_url: &str) -> Result<PathBuf> {
        log::trace!(
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

    fn verify_checksum(&self, plugin_file: &PathBuf, checksum: &str) -> Result<()> {
        let binary_sha256 = file_digest_string(plugin_file).expect("failed to get sha for parcel");
        let verification_sha256 = checksum;
        if binary_sha256 == verification_sha256 {
            log::info!("Package checksum verified successfully");
            Ok(())
        } else {
            Err(anyhow!(
                "Could not validate Checksum, aborting installation"
            ))
        }
    }

    fn add_to_manifest_dir(&self, plugin_manifest: &PluginManifest) -> Result<()> {
        let manifests_dir = self.plugins_dir.join(PLUGIN_MANIFESTS_DIRECTORY_NAME);
        fs::create_dir_all(&manifests_dir)?;
        serde_json::to_writer(
            &File::create(manifests_dir.join(get_manifest_file_name(&plugin_manifest.name())))?,
            plugin_manifest,
        )?;
        log::trace!("Added manifest for {}", &plugin_manifest.name());
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
