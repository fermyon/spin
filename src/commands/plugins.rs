use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use semver::Version;
use spin_plugins::{
    install::{ManifestLocation, PluginInfo, PluginInstaller},
    uninstall::PluginUninstaller,
    PLUGIN_MANIFESTS_DIRECTORY_NAME,
};
use std::path::PathBuf;
use tracing::log;
use url::Url;

const SPIN_PLUGINS_REPO: &str = "https://github.com/fermyon/spin-plugins/";

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
}

impl PluginCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            PluginCommands::Install(cmd) => cmd.run().await,
            PluginCommands::Uninstall(cmd) => cmd.run().await,
            PluginCommands::Upgrade(cmd) => cmd.run().await,
        }
    }
}

/// Install plugins from remote source
#[derive(Parser, Debug)]
pub struct Install {
    /// Name of Spin plugin.
    #[clap(
        name = "PLUGIN_NAME",
        conflicts_with = "REMOTE_PLUGIN_MANIFEST",
        conflicts_with = "LOCAL_PLUGIN_MANIFEST",
        required_unless_present_any = ["REMOTE_PLUGIN_MANIFEST", "LOCAL_PLUGIN_MANIFEST"],
    )]
    pub name: Option<String>,

    /// Path to local plugin manifest.
    #[clap(
        name = "LOCAL_PLUGIN_MANIFEST",
        short = 'f',
        long = "file",
        conflicts_with = "REMOTE_PLUGIN_MANIFEST",
        conflicts_with = "PLUGIN_NAME"
    )]
    pub local_manifest_src: Option<PathBuf>,

    /// Path to remote plugin manifest.
    #[clap(
        name = "REMOTE_PLUGIN_MANIFEST",
        short = 'u',
        long = "url",
        conflicts_with = "LOCAL_PLUGIN_MANIFEST",
        conflicts_with = "PLUGIN_NAME"
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
        conflicts_with = "REMOTE_PLUGIN_MANIFEST",
        conflicts_with = "LOCAL_PLUGIN_MANIFEST",
        requires("PLUGIN_NAME")
    )]
    pub version: Option<Version>,
}

impl Install {
    pub async fn run(self) -> Result<()> {
        let manifest_location = match (self.local_manifest_src, self.remote_manifest_src, self.name) {
            (Some(path), None, None) => ManifestLocation::Local(path),
            (None, Some(url), None) => ManifestLocation::Remote(url),
            (None, None, Some(name)) => ManifestLocation::PluginsRepository(PluginInfo::new(&name, Url::parse(SPIN_PLUGINS_REPO)?, self.version)),
            _ => return Err(anyhow::anyhow!("Must provide plugin name for plugin look up xor remote xor local path to plugin manifest")),
        };
        PluginInstaller::new(
            manifest_location,
            get_spin_plugins_directory()?,
            self.yes_to_all,
            env!("VERGEN_BUILD_SEMVER"),
        )
        .install()
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
        PluginUninstaller::new(&self.name, get_spin_plugins_directory()?).run()?;
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Upgrade {
    /// Name of Spin plugin to upgrade.
    #[clap(
        name = "PLUGIN_NAME",
        conflicts_with = "ALL",
        required_unless_present_any = ["ALL"],
    )]
    pub name: Option<String>,

    /// Upgrade all plugins.
    #[clap(
        short = 'a',
        long = "all",
        name = "ALL",
        conflicts_with = "PLUGIN_NAME",
        conflicts_with = "REMOTE_PLUGIN_MANIFEST",
        conflicts_with = "LOCAL_PLUGIN_MANIFEST",
        takes_value = false
    )]
    pub all: bool,

    /// Path to local plugin manifest.
    #[clap(
        name = "LOCAL_PLUGIN_MANIFEST",
        short = 'f',
        long = "file",
        conflicts_with = "REMOTE_PLUGIN_MANIFEST"
    )]
    pub local_manifest_src: Option<PathBuf>,

    /// Path to remote plugin manifest.
    #[clap(
        name = "REMOTE_PLUGIN_MANIFEST",
        short = 'u',
        long = "url",
        conflicts_with = "LOCAL_PLUGIN_MANIFEST"
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
        conflicts_with = "REMOTE_PLUGIN_MANIFEST",
        conflicts_with = "LOCAL_PLUGIN_MANIFEST",
        conflicts_with = "ALL",
        requires("PLUGIN_NAME")
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
        let plugins_dir = get_spin_plugins_directory()?;
        let spin_version = env!("VERGEN_BUILD_SEMVER");
        let manifest_dir = plugins_dir.join(PLUGIN_MANIFESTS_DIRECTORY_NAME);

        // Check if no plugins are currently installed
        if !manifest_dir.exists() {
            println!("No currently installed plugins to update.");
            return Ok(());
        }

        if self.all {
            // Install the latest of all currently installed plugins
            for plugin in std::fs::read_dir(manifest_dir)? {
                let path = plugin?.path();
                let name = path
                    .file_stem()
                    .ok_or_else(|| anyhow!("expected directory for plugin"))?
                    .to_str()
                    .ok_or_else(|| anyhow!("Cannot convert directory to String"))?
                    .to_string();
                if let Err(e) = PluginInstaller::new(
                    ManifestLocation::PluginsRepository(PluginInfo::new(
                        &name,
                        Url::parse(SPIN_PLUGINS_REPO)?,
                        None,
                    )),
                    plugins_dir.clone(),
                    self.yes_to_all,
                    spin_version,
                )
                .install()
                .await
                {
                    // Ignore plugins that were not installed from the central
                    // plugins repository
                    if e.to_string().contains("Could not find plugin") {
                        log::info!(
                            "Could not update {} plugin as DNE in central repository",
                            name
                        );
                    } else {
                        return Err(e);
                    }
                }
            }
        } else {
            let name = self
                .name
                .ok_or_else(|| anyhow!("plugin name is required for upgrades"))?;
            // If downgrade is allowed, first uninstall the plugin
            if self.downgrade {
                PluginUninstaller::new(&name, plugins_dir.clone()).run()?;
            }
            let manifest_location = match (self.local_manifest_src, self.remote_manifest_src) {
                (Some(path), None) => ManifestLocation::Local(path),
                (None, Some(url)) => ManifestLocation::Remote(url),
                _ => ManifestLocation::PluginsRepository(PluginInfo::new(
                    &name,
                    Url::parse(SPIN_PLUGINS_REPO)?,
                    self.version,
                )),
            };
            PluginInstaller::new(
                manifest_location,
                plugins_dir,
                self.yes_to_all,
                spin_version,
            )
            .install()
            .await?;
        }

        Ok(())
    }
}

/// Gets the path to where Spin plugin are installed.
pub fn get_spin_plugins_directory() -> anyhow::Result<PathBuf> {
    let data_dir = match std::env::var("TEST_PLUGINS_DIRECTORY") {
        Ok(test_dir) => PathBuf::from(test_dir),
        Err(_) => dirs::data_local_dir()
            .or_else(|| dirs::home_dir().map(|p| p.join(".spin")))
            .ok_or_else(|| anyhow!("Unable to get local data directory or home directory"))?,
    };
    let plugins_dir = data_dir.join("spin").join("plugins");
    Ok(plugins_dir)
}
