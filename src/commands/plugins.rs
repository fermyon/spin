use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use semver::Version;
use spin_plugins::install::{ManifestLocation, PluginInfo, PluginInstaller};
use std::path::PathBuf;
use url::Url;

const SPIN_PLUGINS_REPO: &str = "https://github.com/fermyon/spin-plugins/";

/// Install/uninstall plugins
#[derive(Subcommand, Debug)]
pub enum PluginCommands {
    /// Install plugin from the Spin plugin repository.
    ///
    /// The binary or .wasm file of the plugin is copied to the local Spin plugins directory
    /// TODO: consider the ability to install multiple plugins
    Install(Install),

    /// Remove a plugin from your installation.
    Uninstall(Uninstall),
    // TODO: consider Search command

    // TODO: consider List command
}

impl PluginCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            PluginCommands::Install(cmd) => cmd.run().await,
            PluginCommands::Uninstall(cmd) => cmd.run().await,
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
    /// source of local manifest file
    #[clap(
        name = "LOCAL_PLUGIN_MANIFEST",
        short = 'f',
        long = "file",
        conflicts_with = "REMOTE_PLUGIN_MANIFEST",
        conflicts_with = "PLUGIN_NAME"
    )]
    pub local_manifest_src: Option<PathBuf>,
    /// source of remote manifest file
    #[clap(
        name = "REMOTE_PLUGIN_MANIFEST",
        short = 'u',
        long = "url",
        conflicts_with = "LOCAL_PLUGIN_MANIFEST",
        conflicts_with = "PLUGIN_NAME"
    )]
    pub remote_manifest_src: Option<Url>,
    /// skips prompt to accept the installation of the plugin.
    #[clap(short = 'y', long = "yes", takes_value = false)]
    pub yes_to_all: bool,
    /// specify particular version of plugin to install from centralized repository
    #[clap(
        long = "version",
        short = 'v',
        conflicts_with = "REMOTE_PLUGIN_MANIFEST",
        conflicts_with = "LOCAL_PLUGIN_MANIFEST",
        requires("PLUGIN_NAME")
    )]
    /// Specify a particular version of the plugin to be installed from the Centralized Repository
    pub version: Option<Version>,
}

impl Install {
    pub async fn run(self) -> Result<()> {
        println!("Attempting to install plugin: {:?}", self.name);
        let manifest_location = match (self.local_manifest_src, self.remote_manifest_src, self.name) {
            // TODO: move all this parsing into clap to catch input errors.
            (Some(path), None, None) => ManifestLocation::Local(path),
            (None, Some(url), None) => ManifestLocation::Remote(url),
            (None, None, Some(name)) => ManifestLocation::PluginsRepository(PluginInfo::new(name, Url::parse(SPIN_PLUGINS_REPO)?)),
            _ => return Err(anyhow::anyhow!("Must provide plugin name for plugin look up xor remote xor local path to plugin manifest")),
        };
        PluginInstaller::new(
            manifest_location,
            get_spin_plugins_directory()?,
            self.yes_to_all,
        )
        .install()
        .await?;
        Ok(())
    }
}

/// Remove the specified plugin
#[derive(Parser, Debug)]
pub struct Uninstall {
    /// Name of Spin plugin.
    pub name: String,
    // TODO: think about how to handle breaking changes
    // #[structopt(long = "update")]
    // pub update: bool,
}

impl Uninstall {
    pub async fn run(self) -> Result<()> {
        println!("The plugin {:?} will be removed", self.name);
        Ok(())
    }
}

/// Gets the path to where Spin plugin are (to be) installed
pub fn get_spin_plugins_directory() -> anyhow::Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .or_else(|| dirs::home_dir().map(|p| p.join(".spin")))
        .ok_or_else(|| anyhow!("Unable to get local data directory or home directory"))?;
    let plugins_dir = data_dir.join("spin").join("plugins");
    Ok(plugins_dir)
}
