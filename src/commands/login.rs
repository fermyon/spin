use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Result, bail, Context};
use chrono::DateTime;
use chrono::Utc;
use clap::Parser;
use cloud::client::{ConnectionConfig, Client};
use hippo::Client as HippoClient;
use hippo::ConnectionInfo;
use serde::Deserialize;
use serde::Serialize;
use tokio::fs;
use tracing::log;
use uuid::Uuid;

use crate::opts::{BINDLE_SERVER_URL_OPT, BINDLE_URL_ENV, HIPPO_USERNAME, HIPPO_PASSWORD, BINDLE_USERNAME, BINDLE_PASSWORD, INSECURE_OPT, HIPPO_SERVER_URL_OPT, HIPPO_URL_ENV};

// this is the client ID registered in the Cloud's backend
const SPIN_CLIENT_ID: &str = "583e63e9-461f-4fbe-a246-23e0fb1cad10";

const DEFAULT_CLOUD_URL: &str = "http://localhost:5309";

/// Log into the server
#[derive(Parser, Debug)]
#[clap(about = "Log into the server")]
pub struct LoginCommand {
    /// URL of bindle server
    #[clap(
        name = BINDLE_SERVER_URL_OPT,
        long = "bindle-server",
        env = BINDLE_URL_ENV,
        requires = HIPPO_SERVER_URL_OPT,
        requires = HIPPO_USERNAME,
        requires = HIPPO_PASSWORD,
    )]
    pub bindle_server_url: Option<String>,

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
        requires = BINDLE_SERVER_URL_OPT,
        requires = HIPPO_USERNAME,
        requires = HIPPO_PASSWORD,
    )]
    pub hippo_server_url: Option<String>,

    /// Hippo username
    #[clap(
        name = HIPPO_USERNAME,
        long = "hippo-username",
        env = HIPPO_USERNAME,
        requires = BINDLE_SERVER_URL_OPT,
        requires = HIPPO_SERVER_URL_OPT,
        requires = HIPPO_PASSWORD,
    )]
    pub hippo_username: Option<String>,

    /// Hippo password
    #[clap(
        name = HIPPO_PASSWORD,
        long = "hippo-password",
        env = HIPPO_PASSWORD,
        requires = BINDLE_SERVER_URL_OPT,
        requires = HIPPO_SERVER_URL_OPT,
        requires = HIPPO_USERNAME,
    )]
    pub hippo_password: Option<String>,

    /// Display login status
    #[clap(
        name = "status",
        long = "status",
        takes_value = false,
    )]
    pub status: bool,
}

impl LoginCommand {
    pub async fn run(self) -> Result<()> {

        let root = dirs::config_dir().context("Cannot find configuration directory")?.join("spin");

        ensure(&root)?;

        let path = root.join("config.json");

        if self.status {
            let data = fs::read_to_string(path.clone()).await.context(format!("Cannnot display login information"))?;
            let login_connection: LoginConnection = serde_json::from_str(&data)?;

            println!("You are logged into {}", login_connection.url);
            if let Some(bindle_url) = login_connection.bindle_url {
                println!("With a bindle URL of {}", bindle_url);
            }
            let expiration_date = DateTime::parse_from_rfc3339(&login_connection.expiration)?;
            let now: DateTime<Utc> = Utc::now();
            if now > expiration_date {
                println!("Your session has expired.")
            } else {
                println!("Your session will expire on {}.", expiration_date);
            }
            return Ok(());
        }

        if self.hippo_server_url.is_some() {
            // log in with username/password
            let token = match HippoClient::login(
                &HippoClient::new(ConnectionInfo {
                    url: self.hippo_server_url.as_deref().unwrap().to_string(),
                    danger_accept_invalid_certs: self.insecure,
                    api_key: None,
                }),
                self.hippo_username.as_deref().unwrap().to_string(),
                self.hippo_password.as_deref().unwrap().to_string(),
            )
            .await
            {
                Ok(token_info) => token_info,
                Err(err) => bail!(format_login_error(&err)?),
            };

            let login_connection = LoginConnection {
                url: self.hippo_server_url.unwrap().clone(),
                danger_accept_invalid_certs: self.insecure,
                token: token.token.unwrap_or_default(),
                expiration: token.expiration.unwrap_or_default(),
                bindle_url: self.bindle_server_url,
                bindle_username: self.bindle_username,
                bindle_password: self.bindle_password,
            };

            std::fs::write(
                path,
                serde_json::to_string_pretty(&login_connection)?,
            )?;
        } else {
            // log in to the cloud API
            let connection_config = ConnectionConfig {
                url: DEFAULT_CLOUD_URL.to_owned(),
                insecure: self.insecure,
                token: Default::default(),
            };

            let token = github_token(connection_config).await?;

            let login_connection = LoginConnection {
                url: DEFAULT_CLOUD_URL.to_owned(),
                danger_accept_invalid_certs: self.insecure,
                token: token.token.unwrap_or_default(),
                expiration: token.expiration.unwrap_or_default(),
                bindle_url: None,
                bindle_username: None,
                bindle_password: None,
            };

            std::fs::write(
                path,
                serde_json::to_string_pretty(&login_connection)?,
            )?;
        }
        
        Ok(())
    }
}

async fn github_token(connection_config: ConnectionConfig) -> Result<cloud_openapi::models::TokenInfo> {
    let client = Client::new(connection_config);

    // Generate a device code and a user code to activate it with
    let device_code = client
        .create_device_code(Uuid::parse_str(SPIN_CLIENT_ID)?)
        .await?;

    println!(
        "Open {} in your browser",
        device_code.verification_url.clone().unwrap(),
    );

    println!(
        "! Copy your one-time code: {}",
        device_code.user_code.clone().unwrap(),
    );

    // The OAuth library should theoretically handle waiting for the device to be authorized, but
    // testing revealed that it doesn't work. So we manually poll every 10 seconds for fifteen minutes.
    const POLL_INTERVAL_SECS: u64 = 10;
    let mut seconds_elapsed = 0;
    let timeout_seconds = 15 * 60;

    // Loop while waiting for the device code to be authorized by the user
    loop {
        if seconds_elapsed > timeout_seconds {
            bail!("Timed out waiting to authorize the device. Please execute `spin login` again and authorize the device with GitHub.");
        }

        match client.login(device_code.device_code.clone().unwrap()).await {
            Ok(response) => {
                if response.token != None {
                    println!("Device authorized!");
                    return Ok(response);
                }

                println!("Waiting for device authorization...");
                tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                seconds_elapsed += POLL_INTERVAL_SECS;
                continue;
            }
            Err(_) => {
                println!("There was an error while waiting for device authorization");
                tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                seconds_elapsed += POLL_INTERVAL_SECS;
            }
        };
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LoginConnection {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub bindle_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub bindle_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub bindle_password: Option<String>,
    pub danger_accept_invalid_certs: bool,
    pub token: String,
    pub expiration: String,
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