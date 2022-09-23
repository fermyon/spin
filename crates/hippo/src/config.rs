use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
};

use anyhow::{bail, Context, Result};
use hippo_openapi::models::TokenInfo;
use serde::{Deserialize, Serialize};
use tracing::log;

pub const DEFAULT_FERMYON_DIRECTORY: &str = "fermyon";
pub const DEFAULT_CONFIGURATION_FILE: &str = "config.json";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ConnectionInfo {
    pub url: String,
    pub danger_accept_invalid_certs: bool,
    pub token: TokenInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// Root directory for all data and configuration.
    pub root: PathBuf,

    /// Configuration for the connection to the platform.
    pub connection: ConnectionInfo,
}

impl Config {
    pub async fn new(root: Option<PathBuf>) -> Result<Self> {
        let root = match root {
            Some(p) => p.join(DEFAULT_FERMYON_DIRECTORY),
            None => dirs::config_dir()
                .context("Cannot find configuration directory")?
                .join(DEFAULT_FERMYON_DIRECTORY),
        };

        ensure(&root)?;

        let p = root.join(DEFAULT_CONFIGURATION_FILE);
        let connection = match p.exists() {
            true => {
                let cfg_file = File::open(&p).context("cannot open configuration file")?;
                log::trace!("Using configuration file {:?}", &p);
                serde_json::from_reader(cfg_file)
                    .context(format!("Cannot deserialize configuration file {:?}", &p))?
            }
            false => ConnectionInfo::default(),
        };

        Ok(Self { root, connection })
    }

    /// Persist a configuration change.
    pub async fn commit(&self) -> Result<()> {
        let cfg_file = self.root.join(DEFAULT_CONFIGURATION_FILE);
        let f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&cfg_file)?;

        serde_json::to_writer_pretty(f, &self.connection)?;
        tracing::debug!("Configuration saved to {:?}", cfg_file);
        Ok(())
    }
}

/// Ensure the root directory exists, or else create it.
fn ensure(root: &PathBuf) -> Result<()> {
    log::trace!("Ensuring root directory {:?}", root);
    if !root.exists() {
        log::trace!("Creating configuration root directory `{}`", root.display());
        std::fs::create_dir_all(root).with_context(|| {
            format!(
                "Failed to create configuration root directory `{}`",
                root.display()
            )
        })?;
    } else if !root.is_dir() {
        bail!(
            "Configuration root `{}` already exists and is not a directory",
            root.display()
        );
    } else {
        log::trace!(
            "Using existing configuration root directory `{}`",
            root.display()
        );
    }

    Ok(())
}
