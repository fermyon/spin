use crate::opts::*;
use anyhow::{Context, Result};
use bindle::Id;
use core::panic;
use hippo_openapi::apis::{
    account_api::api_account_createtoken_post,
    app_api::{api_app_post, ApiAppPostError},
    channel_api::api_channel_post,
    configuration::{ApiKey, Configuration},
    revision_api::{api_revision_post, ApiRevisionPostError},
    Error,
};
use hippo_openapi::models::{
    ChannelRevisionSelectionStrategy, CreateAppCommand, CreateChannelCommand, CreateTokenCommand,
    RegisterRevisionCommand,
};
use reqwest::header;
use std::path::PathBuf;
use structopt::{clap::AppSettings, StructOpt};

/// Package and upload Spin artifacts, notifying Hippo
#[derive(StructOpt, Debug)]
#[structopt(
    about = "Deploy a Spin application",
    global_settings = &[AppSettings::ColoredHelp, AppSettings::ArgRequiredElseHelp]
)]
pub struct DeployCommand {
    /// Path to spin.toml
    #[structopt(
        name = APP_CONFIG_FILE_OPT,
        short = "f",
        long = "file",
        default_value = "spin.toml"
    )]
    pub app: PathBuf,

    /// URL of bindle server
    #[structopt(
        name = BINDLE_SERVER_URL_OPT,
        long = "bindle-server",
        env = BINDLE_URL_ENV,
    )]
    pub bindle_server_url: String,

    /// Basic http auth username for the bindle server
    #[structopt(
        name = BINDLE_USERNAME,
        long = "bindle-username",
        env = BINDLE_USERNAME,
        requires = BINDLE_PASSWORD
    )]
    pub bindle_username: Option<String>,

    /// Basic http auth password for the bindle server
    #[structopt(
        name = BINDLE_PASSWORD,
        long = "bindle-password",
        env = BINDLE_PASSWORD,
        requires = BINDLE_USERNAME
    )]
    pub bindle_password: Option<String>,

    /// Ignore server certificate errors from bindle and hippo
    #[structopt(
        name = INSECURE_OPT,
        short = "k",
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// URL of hippo server
    #[structopt(
        name = HIPPO_SERVER_URL_OPT,
        long = "hippo-server",
        env = HIPPO_URL_ENV,
    )]
    pub hippo_server_url: String,

    /// Path to assemble the bindle before pushing (defaults to
    /// a temporary directory)
    #[structopt(
        name = STAGING_DIR_OPT,
        long = "staging-dir",
        short = "-d", 
    )]
    pub staging_dir: Option<PathBuf>,

    /// Hippo username
    #[structopt(
        name = "HIPPO_USERNAME",
        long = "hippo-username",
        env = "HIPPO_USERNAME"
    )]
    pub hippo_username: String,

    /// Hippo password
    #[structopt(
        name = "HIPPO_PASSWORD",
        long = "hippo-password",
        env = "HIPPO_PASSWORD"
    )]
    pub hippo_password: String,
}

impl DeployCommand {
    pub async fn run(self) -> Result<()> {
        let bindle_id = self
            .create_and_push_bindle()
            .await
            .expect("Unable to create and push bindle from Spin app");

        let hippo_client_config = self.create_hippo_client_config().await;

        let app_id = self
            .create_hippo_app(&hippo_client_config, &bindle_id)
            .await
            .expect("Unable to create Hippo App");

        self.create_spin_deploy_channel(&hippo_client_config, app_id.as_ref())
            .await
            .expect("Unable to create Hippo Channel");

        self.deploy(&hippo_client_config, &bindle_id).await?;
        println!("Successfully deployed application! See Traefik dashboard for IP address of app.");

        Ok(())
    }

    async fn deploy(
        &self,
        hippo_client_config: &Configuration,
        bindle_id: &Id,
    ) -> Result<(), Error<ApiRevisionPostError>> {
        api_revision_post(
            hippo_client_config,
            Some(RegisterRevisionCommand {
                app_storage_id: bindle_id.name().to_string(),
                revision_number: bindle_id.version_string(),
            }),
        )
        .await
    }

    async fn create_spin_deploy_channel(
        &self,
        hippo_client_config: &Configuration,
        app_id: &str,
    ) -> Result<String, anyhow::Error> {
        api_channel_post(
            hippo_client_config,
            Some(CreateChannelCommand {
                app_id: app_id.to_string(),
                name: "spin-deploy".to_string(),
                domain: None,
                revision_selection_strategy: ChannelRevisionSelectionStrategy::_0,
                range_rule: None,
                active_revision_id: None,
                certificate_id: None,
            }),
        )
        .await
        .map_err(|e| match e {
            Error::ResponseError(r) => {
                anyhow::anyhow!("Unable to create Hippo App: {}", r.content)
            }
            _ => anyhow::anyhow!("Unable to create hippo app"),
        })
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

    async fn create_hippo_app(
        &self,
        hippo_client_config: &Configuration,
        bindle_id: &Id,
    ) -> Result<String, Error<ApiAppPostError>> {
        let app_name = bindle_id.name().to_string();
        let bindle_storage_id = bindle_id.name().to_string();

        api_app_post(
            hippo_client_config,
            Some(CreateAppCommand {
                name: app_name.clone(),
                storage_id: bindle_storage_id.clone(),
            }),
        )
        .await
    }

    async fn create_hippo_client_config(&self) -> Configuration {
        let mut hippo_client_config = Configuration {
            base_path: self.hippo_server_url.clone(),
            ..Default::default()
        };

        hippo_client_config.base_path = self.hippo_server_url.clone();

        let mut headers = header::HeaderMap::new();
        headers.insert(header::ACCEPT, JSON_MIME_TYPE.parse().unwrap());
        headers.insert(header::CONTENT_TYPE, JSON_MIME_TYPE.parse().unwrap());

        hippo_client_config.client = reqwest::Client::builder()
            .danger_accept_invalid_certs(self.insecure)
            .default_headers(headers)
            .build()
            .unwrap();

        match self.get_hippo_token(&hippo_client_config).await {
            Ok(t) => {
                hippo_client_config.api_key = Some(ApiKey {
                    prefix: Some("Bearer".to_owned()),
                    key: t,
                });
            }
            Err(e) => panic!("Unable to log into Hippo: {}", e),
        }
        hippo_client_config
    }

    // Do the auth dance
    // if username/password provided: (1) request token (2) use token to set API_KEY
    // TODO: if username/password not provided: Check file for token
    //      (1) If token is not expired, use.
    //      (2) If token is expired, prompt user for basic auth, request token, use token.

    async fn get_hippo_token(&self, hippo_client_config: &Configuration) -> Result<String> {
        let token = api_account_createtoken_post(
            hippo_client_config,
            Some(CreateTokenCommand {
                user_name: self.hippo_username.clone(),
                password: self.hippo_password.clone(),
            }),
        )
        .await?
        .token;

        match token {
            Some(t) => Ok(t),
            None => panic!("Unable to log into Hippo"),
        }
    }
}
