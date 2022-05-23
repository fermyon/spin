use anyhow::anyhow;
use anyhow::{Context, Result};
use bindle::Id;
use clap::Parser;
use hippo::{Client, ConnectionInfo};
use hippo_openapi::models::ChannelRevisionSelectionStrategy;
use std::path::PathBuf;

use crate::opts::*;

/// Package and upload Spin artifacts, notifying Hippo
#[derive(Parser, Debug)]
#[clap(about = "Deploy a Spin application")]
pub struct DeployCommand {
    /// Path to spin.toml
    #[clap(
        name = APP_CONFIG_FILE_OPT,
        short = 'f',
        long = "file",
        default_value = "spin.toml"
    )]
    pub app: PathBuf,

    /// URL of bindle server
    #[clap(
        name = BINDLE_SERVER_URL_OPT,
        long = "bindle-server",
        env = BINDLE_URL_ENV,
    )]
    pub bindle_server_url: String,

    /// Basic http auth username for the bindle server
    #[clap(
        name = BINDLE_USERNAME,
        long = "bindle-username",
        env = BINDLE_USERNAME,
        requires = BINDLE_PASSWORD
    )]
    pub bindle_username: Option<String>,

    /// Basic http auth password for the bindle server
    #[clap(
        name = BINDLE_PASSWORD,
        long = "bindle-password",
        env = BINDLE_PASSWORD,
        requires = BINDLE_USERNAME
    )]
    pub bindle_password: Option<String>,

    /// Ignore server certificate errors from bindle and hippo
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// URL of hippo server
    #[clap(
        name = HIPPO_SERVER_URL_OPT,
        long = "hippo-server",
        env = HIPPO_URL_ENV,
    )]
    pub hippo_server_url: String,

    /// Path to assemble the bindle before pushing (defaults to
    /// a temporary directory)
    #[clap(
        name = STAGING_DIR_OPT,
        long = "staging-dir",
        short = 'd',
    )]
    pub staging_dir: Option<PathBuf>,

    /// Hippo username
    #[clap(
        name = "HIPPO_USERNAME",
        long = "hippo-username",
        env = "HIPPO_USERNAME"
    )]
    pub hippo_username: String,

    /// Hippo password
    #[clap(
        name = "HIPPO_PASSWORD",
        long = "hippo-password",
        env = "HIPPO_PASSWORD"
    )]
    pub hippo_password: String,
}

impl DeployCommand {
    pub async fn run(self) -> Result<()> {
        let bindle_id = self.create_and_push_bindle().await?;

        let token = match Client::login(
            &Client::new(ConnectionInfo {
                url: self.hippo_server_url.clone(),
                danger_accept_invalid_certs: self.insecure,
                api_key: None,
            }),
            self.hippo_username,
            self.hippo_password,
        )
        .await?
        .token
        {
            Some(t) => t,
            None => String::from(""),
        };

        let hippo_client = Client::new(ConnectionInfo {
            url: self.hippo_server_url.clone(),
            danger_accept_invalid_certs: self.insecure,
            api_key: Some(token),
        });

        let name = bindle_id.name().to_string();

        let app_id = match Client::add_app(&hippo_client, name.clone(), name.clone()).await {
            Ok(id) => id,
            Err(e) => {
                return Err(anyhow!(
                    "Error creating Hippo app called {}: {}",
                    name.clone(),
                    e
                ))
            }
        };

        Client::add_channel(
            &hippo_client,
            app_id,
            name.clone(),
            None,
            ChannelRevisionSelectionStrategy::UseRangeRule,
            None,
            None,
            None,
        )
        .await?;

        Client::add_revision(&hippo_client, name.clone(), bindle_id.version_string()).await?;
        println!("Successfully deployed application!");

        Ok(())
    }

    async fn create_and_push_bindle(&self) -> Result<Id> {
        let source_dir = crate::app_dir(&self.app)?;
        let bindle_connection_info = spin_publish::BindleConnectionInfo::new(
            &self.bindle_server_url,
            self.insecure,
            self.bindle_username.clone(),
            self.bindle_password.clone(),
        );

        let temp_dir = tempfile::tempdir()?;
        let dest_dir = match &self.staging_dir {
            None => temp_dir.path(),
            Some(path) => path.as_path(),
        };
        let (invoice, sources) = spin_publish::expand_manifest(&self.app, None, &dest_dir)
            .await
            .with_context(|| format!("Failed to expand '{}' to a bindle", self.app.display()))?;

        let bindle_id = &invoice.bindle.id;

        spin_publish::write(&source_dir, &dest_dir, &invoice, &sources)
            .await
            .with_context(|| crate::write_failed_msg(bindle_id, dest_dir))?;

        spin_publish::push_all(&dest_dir, bindle_id, bindle_connection_info)
            .await
            .context("Failed to push bindle to server")?;

        Ok(bindle_id.clone())
    }
}
