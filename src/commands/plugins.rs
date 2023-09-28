// Needed for clap derive: https://github.com/clap-rs/clap/issues/4857
#![allow(clippy::almost_swapped)]

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use semver::Version;
use spin_plugins::{
    error::Error,
    lookup::{fetch_plugins_repo, plugins_repo_url, PluginLookup},
    manager::{self, InstallAction, ManifestLocation, PluginManager},
    manifest::{PluginManifest, PluginPackage},
};
use std::path::{Path, PathBuf};
use tracing::log;
use url::Url;

use crate::build_info::*;
use crate::opts::*;

/// Install/uninstall Spin plugins.
#[derive(Subcommand, Debug)]
pub enum PluginCommands {
    /// Install plugin from a manifest.
    ///
    /// The binary file and manifest of the plugin is copied to the local Spin
    /// plugins directory.
    Install(Install),

    /// List available or installed plugins.
    List(List),

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
            PluginCommands::List(cmd) => cmd.run().await,
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

    /// Overrides a failed compatibility check of the plugin with the current version of Spin.
    #[clap(long = PLUGIN_OVERRIDE_COMPATIBILITY_CHECK_FLAG, takes_value = false)]
    pub override_compatibility_check: bool,

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
    pub async fn run(&self) -> Result<()> {
        let manifest_location = match (&self.local_manifest_src, &self.remote_manifest_src, &self.name) {
            (Some(path), None, None) => ManifestLocation::Local(path.to_path_buf()),
            (None, Some(url), None) => ManifestLocation::Remote(url.clone()),
            (None, None, Some(name)) => ManifestLocation::PluginsRepository(PluginLookup::new(name, self.version.clone())),
            _ => return Err(anyhow::anyhow!("For plugin lookup, must provide exactly one of: plugin name, url to manifest, local path to manifest")),
        };
        let manager = PluginManager::try_default()?;
        // Downgrades are only allowed via the `upgrade` subcommand
        let downgrade = false;
        let manifest = manager
            .get_manifest(
                &manifest_location,
                self.override_compatibility_check,
                SPIN_VERSION,
            )
            .await?;
        try_install(
            &manifest,
            &manager,
            self.yes_to_all,
            self.override_compatibility_check,
            downgrade,
            &manifest_location,
        )
        .await?;
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
        let manager = PluginManager::try_default()?;
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
        required_unless_present_any = [PLUGIN_ALL_OPT, PLUGIN_REMOTE_PLUGIN_MANIFEST_OPT, PLUGIN_LOCAL_PLUGIN_MANIFEST_OPT],
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

    /// Overrides a failed compatibility check of the plugin with the current version of Spin.
    #[clap(long = PLUGIN_OVERRIDE_COMPATIBILITY_CHECK_FLAG, takes_value = false)]
    pub override_compatibility_check: bool,

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
        let manager = PluginManager::try_default()?;
        let manifests_dir = manager.store().installed_manifests_directory();

        // Check if no plugins are currently installed
        if !manifests_dir.exists() {
            println!("No currently installed plugins to upgrade.");
            return Ok(());
        }

        if self.all {
            self.upgrade_all(manifests_dir).await
        } else {
            self.upgrade_one().await
        }
    }

    // Install the latest of all currently installed plugins
    async fn upgrade_all(&self, manifests_dir: impl AsRef<Path>) -> Result<()> {
        let manager = PluginManager::try_default()?;
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
            let manifest = match manager
                .get_manifest(
                    &manifest_location,
                    self.override_compatibility_check,
                    SPIN_VERSION,
                )
                .await
            {
                Err(Error::NotFound(e)) => {
                    log::info!("Could not upgrade plugin '{name}': {e:?}");
                    continue;
                }
                Err(e) => return Err(e.into()),
                Ok(m) => m,
            };
            try_install(
                &manifest,
                &manager,
                self.yes_to_all,
                self.override_compatibility_check,
                self.downgrade,
                &manifest_location,
            )
            .await?;
        }
        Ok(())
    }

    async fn upgrade_one(self) -> Result<()> {
        let manager = PluginManager::try_default()?;
        let manifest_location = match (self.local_manifest_src, self.remote_manifest_src) {
            (Some(path), None) => ManifestLocation::Local(path),
            (None, Some(url)) => ManifestLocation::Remote(url),
            _ => ManifestLocation::PluginsRepository(PluginLookup::new(
                self.name
                    .as_ref()
                    .context("plugin name is required for upgrades")?,
                self.version,
            )),
        };
        let manifest = manager
            .get_manifest(
                &manifest_location,
                self.override_compatibility_check,
                SPIN_VERSION,
            )
            .await?;
        try_install(
            &manifest,
            &manager,
            self.yes_to_all,
            self.override_compatibility_check,
            self.downgrade,
            &manifest_location,
        )
        .await?;
        Ok(())
    }
}

/// Install plugins from remote source
#[derive(Parser, Debug)]
pub struct List {
    /// List only installed plugins.
    #[clap(long = "installed", takes_value = false)]
    pub installed: bool,

    /// Filter the list to plugins containing this string.
    #[clap(long = "filter")]
    pub filter: Option<String>,
}

impl List {
    pub async fn run(self) -> Result<()> {
        let mut plugins = if self.installed {
            Self::list_installed_plugins()
        } else {
            Self::list_catalogue_and_installed_plugins().await
        }?;

        plugins.sort_by(|p, q| p.cmp(q));

        if let Some(filter) = self.filter.as_ref() {
            plugins.retain(|p| p.name.contains(filter));
        }

        Self::print(&plugins);
        Ok(())
    }

    fn list_installed_plugins() -> Result<Vec<PluginDescriptor>> {
        let manager = PluginManager::try_default()?;
        let store = manager.store();
        let manifests = store.installed_manifests()?;
        let descriptors = manifests
            .into_iter()
            .map(|m| PluginDescriptor {
                name: m.name(),
                version: m.version().to_owned(),
                installed: true,
                compatibility: PluginCompatibility::for_current(&m),
                manifest: m,
            })
            .collect();
        Ok(descriptors)
    }

    async fn list_catalogue_plugins() -> Result<Vec<PluginDescriptor>> {
        if update_silent().await.is_err() {
            terminal::warn!("Couldn't update plugins registry cache - using most recent");
        }

        let manager = PluginManager::try_default()?;
        let store = manager.store();
        let manifests = store.catalogue_manifests();
        let descriptors = manifests?
            .into_iter()
            .map(|m| PluginDescriptor {
                name: m.name(),
                version: m.version().to_owned(),
                installed: m.is_installed_in(store),
                compatibility: PluginCompatibility::for_current(&m),
                manifest: m,
            })
            .collect();
        Ok(descriptors)
    }

    async fn list_catalogue_and_installed_plugins() -> Result<Vec<PluginDescriptor>> {
        let catalogue = Self::list_catalogue_plugins().await?;
        let installed = Self::list_installed_plugins()?;
        Ok(merge_plugin_lists(catalogue, installed))
    }

    fn print(plugins: &[PluginDescriptor]) {
        if plugins.is_empty() {
            println!("No plugins found");
        } else {
            for p in plugins {
                let installed = if p.installed { " [installed]" } else { "" };
                let compat = match &p.compatibility {
                    PluginCompatibility::Compatible => String::new(),
                    PluginCompatibility::IncompatibleSpin(v) => format!(" [requires Spin {v}]"),
                    PluginCompatibility::Incompatible => String::from(" [incompatible]"),
                };
                println!("{} {}{}{}", p.name, p.version, installed, compat);
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum PluginCompatibility {
    Compatible,
    IncompatibleSpin(String),
    Incompatible,
}

impl PluginCompatibility {
    pub(crate) fn for_current(manifest: &PluginManifest) -> Self {
        if manifest.has_compatible_package() {
            let spin_version = SPIN_VERSION;
            if manifest.is_compatible_spin_version(spin_version) {
                Self::Compatible
            } else {
                Self::IncompatibleSpin(manifest.spin_compatibility())
            }
        } else {
            Self::Incompatible
        }
    }
}

#[derive(Debug)]
struct PluginDescriptor {
    name: String,
    version: String,
    compatibility: PluginCompatibility,
    installed: bool,
    manifest: PluginManifest,
}

impl PluginDescriptor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let version_cmp = match (
            semver::Version::parse(&self.version),
            semver::Version::parse(&other.version),
        ) {
            (Ok(v1), Ok(v2)) => v1.cmp(&v2),
            _ => self.version.cmp(&other.version),
        };

        self.name.cmp(&other.name).then(version_cmp)
    }
}

fn merge_plugin_lists(a: Vec<PluginDescriptor>, b: Vec<PluginDescriptor>) -> Vec<PluginDescriptor> {
    let mut result = a;

    for descriptor in b {
        // Use the manifest for sameness checking, because an installed local build could have the same name
        // and version as a registry package, yet be a different binary. It could even have different
        // compatibility characteristics!
        let already_got = result
            .iter()
            .any(|desc| desc.manifest == descriptor.manifest);
        if !already_got {
            result.push(descriptor);
        }
    }

    result
}

/// Updates the locally cached spin-plugins repository, fetching the latest plugins.
pub(crate) async fn update() -> Result<()> {
    update_silent().await?;
    println!("Plugin information updated successfully");
    Ok(())
}

pub(crate) async fn update_silent() -> Result<()> {
    let manager = PluginManager::try_default()?;

    let mut locker = manager.update_lock().await;
    let guard = locker.lock_updates();
    if guard.denied() {
        anyhow::bail!("Another plugin update operation is already in progress");
    }

    let plugins_dir = manager.store().get_plugins_directory();
    let url = plugins_repo_url()?;
    fetch_plugins_repo(&url, plugins_dir, true).await?;
    Ok(())
}

fn continue_to_install(
    manifest: &PluginManifest,
    package: &PluginPackage,
    yes_to_all: bool,
) -> Result<bool> {
    Ok(yes_to_all || prompt_confirm_install(manifest, package)?)
}

fn prompt_confirm_install(manifest: &PluginManifest, package: &PluginPackage) -> Result<bool> {
    let prompt = format!(
        "Are you sure you want to install plugin '{}' with license {} from {}?",
        manifest.name(),
        manifest.license(),
        package.url()
    );
    let install = dialoguer::Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact_opt()?
        .unwrap_or(false);
    if !install {
        println!("Plugin '{}' will not be installed", manifest.name());
    }
    Ok(install)
}

async fn try_install(
    manifest: &PluginManifest,
    manager: &PluginManager,
    yes_to_all: bool,
    override_compatibility_check: bool,
    downgrade: bool,
    source: &ManifestLocation,
) -> Result<bool> {
    let install_action = manager.check_manifest(
        manifest,
        SPIN_VERSION,
        override_compatibility_check,
        downgrade,
    )?;

    if let InstallAction::NoAction { name, version } = install_action {
        eprintln!("Plugin '{name}' is already installed with version {version}.");
        return Ok(false);
    }

    let package = manager::get_package(manifest)?;
    if continue_to_install(manifest, package, yes_to_all)? {
        let installed = manager.install(manifest, package, source).await?;
        println!("Plugin '{installed}' was installed successfully!");

        if let Some(description) = manifest.description() {
            println!("\nDescription:");
            println!("\t{description}");
        }

        if let Some(homepage) = manifest.homepage_url().filter(|h| h.scheme() == "https") {
            println!("\nHomepage:");
            println!("\t{homepage}");
        }

        Ok(true)
    } else {
        Ok(false)
    }
}
