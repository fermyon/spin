use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Url;
use spin_app::locked::LockedApp;
use spin_trigger::cli::{SPIN_LOCKED_URL, SPIN_WORKING_DIR};

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::opts::*;

/// Commands for working with OCI registries to distribute applications.
/// The set of commands for OCI is EXPERIMENTAL, and may change in future versions of Spin.
/// Currently, the OCI commands are reusing the credentials from ~/.docker/config.json to
/// authenticate to registries.
#[derive(Subcommand, Debug)]
pub enum RegistryCommands {
    /// Push a Spin application to an OCI registry.
    Push(Push),
    /// Pull a Spin application from an OCI registry.
    Pull(Pull),
    /// Run a Spin application from an OCI registry.
    Run(Run),
}

impl RegistryCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            RegistryCommands::Push(cmd) => cmd.run().await,
            RegistryCommands::Pull(cmd) => cmd.run().await,
            RegistryCommands::Run(cmd) => cmd.run().await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct Push {
    /// Path to spin.toml
    #[clap(
        name = APP_CONFIG_FILE_OPT,
        short = 'f',
        long = "file",
    )]
    pub app: Option<PathBuf>,

    /// Ignore server certificate errors
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// Reference of the Spin application
    #[clap()]
    pub reference: String,
}

impl Push {
    pub async fn run(self) -> Result<()> {
        let app_file = self
            .app
            .as_deref()
            .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());

        let dir = tempfile::tempdir()?;
        let app = spin_loader::local::from_file(&app_file, Some(dir.path()), &None).await?;

        let mut client = spin_publish::oci::client::Client::new(self.insecure, None).await?;
        client.push(&app, &self.reference).await?;
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Pull {
    /// Ignore server certificate errors
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// Reference of the Spin application
    #[clap()]
    pub reference: String,
}

impl Pull {
    /// Pull a Spin application from an OCI registry
    pub async fn run(self) -> Result<()> {
        let mut client = spin_publish::oci::client::Client::new(self.insecure, None).await?;
        client.pull(&self.reference).await?;

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Run {
    /// Connect to the registry endpoint over HTTP, not HTTPS.
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// Pass an environment variable (key=value) to all components of the application.
    #[clap(short = 'e', long = "env", parse(try_from_str = parse_env_var))]
    pub env: Vec<(String, String)>,

    /// Reference of the Spin application
    #[clap()]
    pub reference: String,

    /// All other args, to be passed through to the trigger
    ///  TODO: The arguments have to be passed like `-- --follow-all` for now.
    #[clap(hide = true)]
    pub trigger_args: Vec<OsString>,
}

impl Run {
    /// Run a Spin application from an OCI registry
    pub async fn run(self) -> Result<()> {
        let mut client = spin_publish::oci::client::Client::new(self.insecure, None).await?;
        client.pull(&self.reference).await?;

        let app_path = client.cache.lockfile_path(&self.reference).await?;
        let working_dir = tempfile::tempdir()?;

        // Read the lockfile from the registry cache and mutate it to add environment variables.
        let mut app: LockedApp = serde_json::from_slice(&tokio::fs::read(&app_path).await?)?;

        // Apply --env to component environments
        if !self.env.is_empty() {
            for component in app.components.iter_mut() {
                component.env.extend(self.env.iter().cloned());
            }
        }

        let trigger_type = &app
            .triggers
            .first()
            .context("application expected to have at least one trigger")?
            .trigger_type;

        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
        cmd.arg("trigger")
            .arg(trigger_type)
            // TODO: This should be inferred from the lockfile.
            .arg("--oci")
            // TODO: Once we figure out how to handle the flags for triggers, i.e. `-- --follow-all`, remove this.
            .arg("--follow-all")
            .args(&self.trigger_args)
            .env(SPIN_WORKING_DIR, working_dir.path());

        let app_path = Self::write_locked_app(&app, working_dir.path()).await?;

        let url = Url::from_file_path(app_path)
            .expect("cannot parse URL from locked app file")
            .to_string();
        cmd.env(SPIN_LOCKED_URL, &url);

        tracing::trace!("Running trigger executor: {:?}", cmd);

        let mut child = cmd.spawn().context("Failed to execute trigger")?;

        // Terminate trigger executor if `spin up` itself receives a termination signal
        #[cfg(not(windows))]
        {
            // https://github.com/nix-rust/nix/issues/656
            let pid = nix::unistd::Pid::from_raw(child.id() as i32);
            ctrlc::set_handler(move || {
                if let Err(err) = nix::sys::signal::kill(pid, nix::sys::signal::SIGTERM) {
                    tracing::warn!("Failed to kill trigger handler process: {:?}", err)
                }
            })?;
        }

        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            bail!(status);
        }
    }

    async fn write_locked_app(app: &LockedApp, working_dir: &Path) -> Result<PathBuf> {
        let path = working_dir.join("spin.lock");
        let contents = serde_json::to_vec(&app)?;
        tokio::fs::write(&path, contents).await?;
        Ok(path)
    }
}

// Parse the environment variables passed in `key=value` pairs.
fn parse_env_var(s: &str) -> Result<(String, String)> {
    let parts: Vec<_> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("Environment variable must be of the form `key=value`");
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
}
