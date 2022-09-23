use anyhow::{bail, Context, Result};
use clap::Parser;
use hippo::{client::Client, config::{ConnectionInfo, Config}};
use serde::{Deserialize, Serialize};

use crate::{sloth::warn_if_slow_response, opts::{BINDLE_SERVER_URL_OPT, BINDLE_URL_ENV, BINDLE_USERNAME, BINDLE_PASSWORD, INSECURE_OPT, HIPPO_SERVER_URL_OPT, HIPPO_URL_ENV}};

use super::CommonOpts;

/// Sign in to Hippo
#[derive(Parser, Debug)]
#[clap(about = "Sign in to Hippo")]
pub struct LoginCommand {
    #[clap(flatten)]
    pub opts: CommonOpts,

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

impl LoginCommand {
    pub async fn run(self) -> Result<()> {
        self.check_hippo_healthz().await?;

        let _sloth_warning = warn_if_slow_response(&self.hippo_server_url);

        let mut hippo_client_config = Config::new(self.opts.dir).await?;
        hippo_client_config.connection = ConnectionInfo {
            url: self.hippo_server_url.clone(),
            danger_accept_invalid_certs: self.insecure,
            token: Default::default(),
        };

        let token = match Client::login(
            &Client::new(hippo_client_config.connection),
            self.hippo_username.clone(),
            self.hippo_password.clone(),
        )
        .await
        {
            Ok(token_info) => token_info,
            Err(err) => bail!(format_login_error(&err)?),
        };

        hippo_client_config.connection = ConnectionInfo {
            danger_accept_invalid_certs: self.insecure,
            token,
            url: self.hippo_server_url,
        };
        hippo_client_config.commit().await?;

        println!("Login successful.");

        Ok(())
    }

    async fn check_hippo_healthz(&self) -> Result<()> {
        let hippo_base_url = url::Url::parse(&self.hippo_server_url)?;
        let hippo_healthz_url = hippo_base_url.join("/healthz")?;
        reqwest::get(hippo_healthz_url.to_string())
            .await?
            .error_for_status()
            .with_context(|| format!("Hippo server {} is unhealthy", hippo_base_url))?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
struct LoginHippoError {
    title: String,
    detail: String,
}

fn format_login_error(err: &anyhow::Error) -> anyhow::Result<String> {
    let error: LoginHippoError = serde_json::from_str(err.to_string().as_str())?;
    if error.detail.ends_with(": ") {
        Ok(format!(
            "Problem logging into Hippo: {}",
            error.detail.replace(": ", ".")
        ))
    } else {
        Ok(format!("Problem logging into Hippo: {}", error.detail))
    }
}
