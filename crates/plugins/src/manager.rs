use crate::{
    error::*,
    lookup::PluginLookup,
    manifest::{warn_unsupported_version, PluginManifest, PluginPackage},
    store::PluginStore,
    SPIN_INTERNAL_COMMANDS,
};

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{copy, Cursor},
    path::{Path, PathBuf},
};
use tempfile::{tempdir, TempDir};
use tracing::log;
use url::Url;

// Url scheme prefix of a plugin that is installed from a local source
const URL_FILE_SCHEME: &str = "file";

/// Location of manifest of the plugin to be installed.
pub enum ManifestLocation {
    /// Plugin manifest can be copied from a local path.
    Local(PathBuf),
    /// Plugin manifest should be pulled from a specific address.
    Remote(Url),
    /// Plugin manifest lives in the centralized plugins repository
    PluginsRepository(PluginLookup),
}

/// Provides accesses to functionality to inspect and manage the installation of plugins.
pub struct PluginManager {
    store: PluginStore,
}

impl PluginManager {
    /// Creates a `PluginManager` with the default install location.
    pub fn try_default() -> anyhow::Result<Self> {
        let store = PluginStore::try_default()?;
        Ok(Self { store })
    }

    /// Returns the underlying store object
    pub fn store(&self) -> &PluginStore {
        &self.store
    }

    /// Installs the Spin plugin with the given manifest If installing a plugin from the centralized
    /// Spin plugins repository, it fetches the latest contents of the repository and searches for
    /// the appropriately named and versioned plugin manifest. Parses the plugin manifest to get the
    /// appropriate source for the machine OS and architecture. Verifies the checksum of the source,
    /// unpacks and installs it into the plugins directory.
    /// Returns name of plugin that was successfully installed.
    pub async fn install(
        &self,
        plugin_manifest: &PluginManifest,
        plugin_package: &PluginPackage,
    ) -> Result<String> {
        let target = plugin_package.url.to_owned();
        let target_url = Url::parse(&target)?;
        let temp_dir = tempdir()?;
        let plugin_tarball_path = match target_url.scheme() {
            URL_FILE_SCHEME => target_url
                .to_file_path()
                .map_err(|_| anyhow!("Invalid file URL: {target_url:?}"))?,
            _ => download_plugin(&plugin_manifest.name(), &temp_dir, &target).await?,
        };
        verify_checksum(&plugin_tarball_path, &plugin_package.sha256)?;

        self.store
            .untar_plugin(&plugin_tarball_path, &plugin_manifest.name())?;

        // Save manifest to installed plugins directory
        self.store.add_manifest(plugin_manifest)?;
        Ok(plugin_manifest.name())
    }

    /// Uninstalls a plugin with a given name, removing it and it's manifest from the local plugins
    /// directory.
    /// Returns true if plugin was successfully uninstalled and false if plugin did not exist.
    pub fn uninstall(&self, plugin_name: &str) -> Result<bool> {
        let plugin_store = self.store();
        let manifest_file = plugin_store.installed_manifest_path(plugin_name);
        let exists = manifest_file.exists();
        if exists {
            // Remove the manifest and the plugin installation directory
            fs::remove_file(manifest_file)?;
            fs::remove_dir_all(plugin_store.plugin_subdirectory_path(plugin_name))?;
        }
        Ok(exists)
    }

    /// Checks manifest to see if the plugin is compatible with the running version of Spin, does
    /// not have a conflicting name with Spin internal commands, and is not a downgrade of a
    /// currently installed plugin.
    pub fn check_manifest(
        &self,
        plugin_manifest: &PluginManifest,
        spin_version: &str,
        override_compatibility_check: bool,
        allow_downgrades: bool,
    ) -> Result<InstallAction> {
        // Disallow installing plugins with the same name as spin internal subcommands
        if SPIN_INTERNAL_COMMANDS
            .iter()
            .any(|&s| s == plugin_manifest.name())
        {
            bail!(
                "Can't install a plugin with the same name ('{}') as an internal command",
                plugin_manifest.name()
            );
        }

        // Disallow reinstalling identical plugins and downgrading unless permitted.
        if let Ok(installed) = self.store.read_plugin_manifest(&plugin_manifest.name()) {
            if &installed == plugin_manifest {
                return Ok(InstallAction::NoAction {
                    name: plugin_manifest.name(),
                    version: installed.version,
                });
            } else if installed.version > plugin_manifest.version && !allow_downgrades {
                bail!(
                    "Newer version {} of plugin '{}' is already installed. To downgrade to version {} set the `--downgrade` flag.",
                    installed.version,
                    plugin_manifest.name(),
                    plugin_manifest.version,
                );
            }
        }

        warn_unsupported_version(plugin_manifest, spin_version, override_compatibility_check)?;

        Ok(InstallAction::Continue)
    }

    /// Fetches a manifest from a local, remote, or repository location and returned the parsed
    /// PluginManifest object.
    pub async fn get_manifest(
        &self,
        manifest_location: &ManifestLocation,
    ) -> PluginLookupResult<PluginManifest> {
        let plugin_manifest = match manifest_location {
            ManifestLocation::Remote(url) => {
                log::info!("Pulling manifest for plugin from {url}");
                reqwest::get(url.as_ref())
                    .await
                    .map_err(|e| {
                        Error::ConnectionFailed(ConnectionFailedError::new(
                            url.as_str().to_string(),
                            e.to_string(),
                        ))
                    })?
                    .error_for_status()
                    .map_err(|e| {
                        Error::ConnectionFailed(ConnectionFailedError::new(
                            url.as_str().to_string(),
                            e.to_string(),
                        ))
                    })?
                    .json::<PluginManifest>()
                    .await
                    .map_err(|e| {
                        Error::InvalidManifest(InvalidManifestError::new(
                            None,
                            url.as_str().to_string(),
                            e.to_string(),
                        ))
                    })?
            }
            ManifestLocation::Local(path) => {
                log::info!("Pulling manifest for plugin from {}", path.display());
                let file = File::open(path).map_err(|e| {
                    Error::NotFound(NotFoundError::new(
                        None,
                        path.display().to_string(),
                        e.to_string(),
                    ))
                })?;
                serde_json::from_reader(file).map_err(|e| {
                    Error::InvalidManifest(InvalidManifestError::new(
                        None,
                        path.display().to_string(),
                        e.to_string(),
                    ))
                })?
            }
            ManifestLocation::PluginsRepository(lookup) => {
                lookup
                    .get_manifest_from_repository(self.store().get_plugins_directory())
                    .await?
            }
        };
        Ok(plugin_manifest)
    }
}

/// The action required to install a plugin to the desired version.
pub enum InstallAction {
    /// The installation needs to continue.
    Continue,
    /// No further action is required. This occurs when the plugin is already at the desired version.
    NoAction { name: String, version: String },
}

/// Gets the appropriate package for the running OS and Arch if exists
pub fn get_package(plugin_manifest: &PluginManifest) -> Result<&PluginPackage> {
    use std::env::consts::{ARCH, OS};
    plugin_manifest
        .packages
        .iter()
        .find(|p| p.os.rust_name() == OS && p.arch.rust_name() == ARCH)
        .ok_or_else(|| {
            anyhow!("This plugin does not support this OS ({OS}) or architecture ({ARCH}).")
        })
}

async fn download_plugin(name: &str, temp_dir: &TempDir, target_url: &str) -> Result<PathBuf> {
    log::trace!("Trying to get tar file for plugin '{name}' from {target_url}");
    let plugin_bin = reqwest::get(target_url).await?;
    let mut content = Cursor::new(plugin_bin.bytes().await?);
    let dir = temp_dir.path();
    let mut plugin_file = dir.join(name);
    plugin_file.set_extension("tar.gz");
    let mut temp_file = File::create(&plugin_file)?;
    copy(&mut content, &mut temp_file)?;
    Ok(plugin_file)
}

fn verify_checksum(plugin_file: &Path, expected_sha256: &str) -> Result<()> {
    let actual_sha256 = file_digest_string(plugin_file)?;
    if actual_sha256 == expected_sha256 {
        log::info!("Package checksum verified successfully");
        Ok(())
    } else {
        Err(anyhow!("Checksum did not match, aborting installation."))
    }
}

fn file_digest_string(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Could not open file at {}", path.display()))?;
    let mut sha = Sha256::new();
    std::io::copy(&mut file, &mut sha)?;
    let digest_value = sha.finalize();
    let digest_string = format!("{:x}", digest_value);
    Ok(digest_string)
}
