use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use semver::Version;
use spin_plugins::{
    fetch_plugins_repo,
    manager::{self, ManifestLocation, PluginManager},
    manifest::{PluginManifest, PluginPackage},
    prompt_confirm_install, PluginLookup, PLUGIN_NOT_FOUND_ERROR_MSG,
};
use std::path::{Path, PathBuf};
use tracing::log;
use url::Url;

use crate::opts::*;

/// Install/uninstall Spin plugins.
#[derive(Subcommand, Debug)]
pub enum PluginCommands {
    /// Install plugin from a manifest.
    ///
    /// The binary file and manifest of the plugin is copied to the local Spin
    /// plugins directory.
    Install(Install),

    /// Remove a plugin from your installation.
    Uninstall(Uninstall),

    /// Upgrade one or all plugins.
    Upgrade(Upgrade),

    /// Fetch the latest Spin plugins from the spin-plugins repository.
    Update,
}

impl PluginCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            PluginCommands::Install(cmd) => cmd.run().await,
            PluginCommands::Uninstall(cmd) => cmd.run().await,
            PluginCommands::Upgrade(cmd) => cmd.run().await,
            PluginCommands::Update => update().await,
        }
    }
}

/// Install plugins from remote source
#[derive(Parser, Debug)]
pub struct Install {
    /// Name of Spin plugin.
    #[clap(
        name = PLUGIN_NAME_OPT,
        conflicts_with = PLUGIN_REMOTE_PLUGIN_MANIFEST_OPT,
        conflicts_with = PLUGIN_LOCAL_PLUGIN_MANIFEST_OPT,
        required_unless_present_any = [PLUGIN_REMOTE_PLUGIN_MANIFEST_OPT, PLUGIN_LOCAL_PLUGIN_MANIFEST_OPT],
    )]
    pub name: Option<String>,

    /// Path to local plugin manifest.
    #[clap(
        name = PLUGIN_LOCAL_PLUGIN_MANIFEST_OPT,
        short = 'f',
        long = "file",
        conflicts_with = PLUGIN_REMOTE_PLUGIN_MANIFEST_OPT,
        conflicts_with = PLUGIN_NAME_OPT,
    )]
    pub local_manifest_src: Option<PathBuf>,

    /// URL of remote plugin manifest to install.
    #[clap(
        name = PLUGIN_REMOTE_PLUGIN_MANIFEST_OPT,
        short = 'u',
        long = "url",
        conflicts_with = PLUGIN_LOCAL_PLUGIN_MANIFEST_OPT,
        conflicts_with = PLUGIN_NAME_OPT,
    )]
    pub remote_manifest_src: Option<Url>,

    /// Skips prompt to accept the installation of the plugin.
    #[clap(short = 'y', long = "yes", takes_value = false)]
    pub yes_to_all: bool,

    /// Specific version of a plugin to be install from the centralized plugins
    /// repository.
    #[clap(
        long = "version",
        short = 'v',
        conflicts_with = PLUGIN_REMOTE_PLUGIN_MANIFEST_OPT,
        conflicts_with = PLUGIN_LOCAL_PLUGIN_MANIFEST_OPT,
        requires(PLUGIN_NAME_OPT)
    )]
    pub version: Option<Version>,
}

impl Install {
    pub async fn run(self) -> Result<()> {
        let manifest_location = match (self.local_manifest_src, self.remote_manifest_src, self.name) {
            (Some(path), None, None) => ManifestLocation::Local(path),
            (None, Some(url), None) => ManifestLocation::Remote(url),
            (None, None, Some(name)) => ManifestLocation::PluginsRepository(PluginLookup::new(&name, self.version)),
            _ => return Err(anyhow::anyhow!("For plugin lookup, must provide exactly one of: plugin name, url to manifest, local path to manifest")),
        };
        let manager = PluginManager::default()?;
        // Downgrades are only allowed via the `upgrade` subcommand
        let downgrade = false;
        try_install(&manifest_location, &manager, self.yes_to_all, downgrade).await?;
        Ok(())
    }
}

/// Uninstalls specified plugin.
#[derive(Parser, Debug)]
pub struct Uninstall {
    /// Name of Spin plugin.
    pub name: String,
}

impl Uninstall {
    pub async fn run(self) -> Result<()> {
        let manager = PluginManager::default()?;
        let uninstalled = manager.uninstall(&self.name)?;
        if uninstalled {
            println!("Plugin {} was successfully uninstalled", self.name);
        } else {
            println!(
                "Plugin {} isn't present, so no changes were made",
                self.name
            );
        }
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Upgrade {
    /// Name of Spin plugin to upgrade.
    #[clap(
        name = PLUGIN_NAME_OPT,
        conflicts_with = PLUGIN_ALL_OPT,
        required_unless_present_any = [PLUGIN_ALL_OPT],
    )]
    pub name: Option<String>,

    /// Upgrade all plugins.
    #[clap(
        short = 'a',
        long = "all",
        name = PLUGIN_ALL_OPT,
        conflicts_with = PLUGIN_NAME_OPT,
        conflicts_with = PLUGIN_REMOTE_PLUGIN_MANIFEST_OPT,
        conflicts_with = PLUGIN_LOCAL_PLUGIN_MANIFEST_OPT,
        takes_value = false,
    )]
    pub all: bool,

    /// Path to local plugin manifest.
    #[clap(
        name = PLUGIN_LOCAL_PLUGIN_MANIFEST_OPT,
        short = 'f',
        long = "file",
        conflicts_with = PLUGIN_REMOTE_PLUGIN_MANIFEST_OPT,
    )]
    pub local_manifest_src: Option<PathBuf>,

    /// Path to remote plugin manifest.
    #[clap(
        name = PLUGIN_REMOTE_PLUGIN_MANIFEST_OPT,
        short = 'u',
        long = "url",
        conflicts_with = PLUGIN_LOCAL_PLUGIN_MANIFEST_OPT,
    )]
    pub remote_manifest_src: Option<Url>,

    /// Skips prompt to accept the installation of the plugin[s].
    #[clap(short = 'y', long = "yes", takes_value = false)]
    pub yes_to_all: bool,

    /// Specific version of a plugin to be install from the centralized plugins
    /// repository.
    #[clap(
        long = "version",
        short = 'v',
        conflicts_with = PLUGIN_REMOTE_PLUGIN_MANIFEST_OPT,
        conflicts_with = PLUGIN_LOCAL_PLUGIN_MANIFEST_OPT,
        conflicts_with = PLUGIN_ALL_OPT,
        requires(PLUGIN_NAME_OPT)
    )]
    pub version: Option<Version>,

    /// Allow downgrading a plugin's version.
    #[clap(short = 'd', long = "downgrade", takes_value = false)]
    pub downgrade: bool,
}

impl Upgrade {
    /// Upgrades one or all plugins by reinstalling the latest or a specified
    /// version of a plugin. If downgrade is specified, first uninstalls the
    /// plugin.
    pub async fn run(self) -> Result<()> {
        let manager = PluginManager::default()?;
        let manifests_dir = manager.store().installed_manifests_directory();

        // Check if no plugins are currently installed
        if !manifests_dir.exists() {
            println!("No currently installed plugins to upgrade.");
            return Ok(());
        }

        if self.all {
            self.upgrade_all(manifests_dir).await
        } else {
            let plugin_name = self
                .name
                .clone()
                .ok_or_else(|| anyhow!("plugin name is required for upgrades"))?;
            self.upgrade_one(&plugin_name).await
        }
    }

    // Install the latest of all currently installed plugins
    async fn upgrade_all(&self, manifests_dir: impl AsRef<Path>) -> Result<()> {
        let manager = PluginManager::default()?;
        for plugin in std::fs::read_dir(manifests_dir)? {
            let path = plugin?.path();
            let name = path
                .file_stem()
                .ok_or_else(|| anyhow!("No stem for path {}", path.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Cannot convert path {} stem to str", path.display()))?
                .to_string();
            let manifest_location =
                ManifestLocation::PluginsRepository(PluginLookup::new(&name, None));
            if let Err(e) = try_install(
                &manifest_location,
                &manager,
                self.yes_to_all,
                self.downgrade,
            )
            .await
            {
                // Ignore plugins that were not installed from the central
                // plugins repository
                if e.to_string().contains(PLUGIN_NOT_FOUND_ERROR_MSG) {
                    log::info!(
                        "Could not upgrade {} plugin as does not exist in spin-plugins repository",
                        name
                    );
                } else {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    async fn upgrade_one(self, name: &str) -> Result<()> {
        let manager = PluginManager::default()?;
        let manifest_location = match (self.local_manifest_src, self.remote_manifest_src) {
            (Some(path), None) => ManifestLocation::Local(path),
            (None, Some(url)) => ManifestLocation::Remote(url),
            _ => ManifestLocation::PluginsRepository(PluginLookup::new(name, self.version)),
        };
        try_install(
            &manifest_location,
            &manager,
            self.yes_to_all,
            self.downgrade,
        )
        .await?;
        Ok(())
    }
}

/// Updates the locally cached spin-plugins repository, fetching the latest plugins.
async fn update() -> Result<()> {
    let manager = PluginManager::default()?;
    let plugins_dir = manager.store().get_plugins_directory();
    fetch_plugins_repo(plugins_dir, true).await
}

fn continue_to_install(
    manifest: &PluginManifest,
    package: &PluginPackage,
    yes_to_all: bool,
) -> Result<bool> {
    Ok(yes_to_all || prompt_confirm_install(manifest, package)?)
}

async fn try_install(
    manifest_location: &ManifestLocation,
    manager: &PluginManager,
    yes_to_all: bool,
    downgrade: bool,
) -> Result<bool> {
    let spin_version = env!("VERGEN_BUILD_SEMVER");
    let manifest = manager.get_manifest(manifest_location).await?;
    manager.check_manifest(&manifest, spin_version, downgrade)?;
    let package = manager::get_package(&manifest)?;
    if !continue_to_install(&manifest, package, yes_to_all)? {
        Ok(false)
    } else {
        let installed = manager.install(&manifest, package).await?;
        println!("Plugin {} was installed successfully!", installed);
        Ok(true)
    }
}
