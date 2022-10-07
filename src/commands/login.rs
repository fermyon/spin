use std::io::stdin;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Parser;
use cloud::client::{Client, ConnectionConfig};
use cloud_openapi::models::DeviceCodeItem;
use cloud_openapi::models::TokenInfo;
use hippo::Client as HippoClient;
use hippo::ConnectionInfo;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use tokio::fs;
use tracing::log;
use uuid::Uuid;

use crate::opts::{
    BINDLE_PASSWORD, BINDLE_SERVER_URL_OPT, BINDLE_URL_ENV, BINDLE_USERNAME, HIPPO_PASSWORD,
    HIPPO_SERVER_URL_OPT, HIPPO_URL_ENV, HIPPO_USERNAME, INSECURE_OPT,
};

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
    #[clap(name = "status", long = "status", takes_value = false)]
    pub status: bool,

    // fetch a device code
    #[clap(
        name = "get-device-code",
        long = "get-device-code",
        takes_value = false
    )]
    pub get_device_code: bool,

    // check a device code
    #[clap(
        name = "check-device-code",
        long = "check-device-code",
        takes_value = false
    )]
    pub check_device_code: Option<String>,
}

impl LoginCommand {
    pub async fn run(self) -> Result<()> {
        let root = dirs::config_dir()
            .context("Cannot find configuration directory")?
            .join("spin");

        ensure(&root)?;

        let path = root.join("config.json");

        if self.status {
            let data = fs::read_to_string(path.clone())
                .await
                .context("Cannnot display login information")?;
            println!("{}", data);
            return Ok(());
        }

        if let Some(url) = self.hippo_server_url {
            let login_connection: LoginConnection;
            // prompt the user for the authentication method
            let auth_method = prompt_for_auth_method();
            if auth_method == AuthMethod::UsernameAndPassword {
                let username = match self.hippo_username {
                    Some(username) => username,
                    None => {
                        print!("Hippo username: ");
                        let mut input = String::new();
                        stdin()
                            .read_line(&mut input)
                            .expect("unable to read user input");
                        input.trim().to_owned()
                    }
                };
                let password = match self.hippo_password {
                    Some(password) => password,
                    None => {
                        print!("Hippo pasword: ");
                        rpassword::read_password()
                            .expect("unable to read user input")
                            .trim()
                            .to_owned()
                    }
                };
                // log in with username/password
                let token = match HippoClient::login(
                    &HippoClient::new(ConnectionInfo {
                        url: url.clone(),
                        danger_accept_invalid_certs: self.insecure,
                        api_key: None,
                    }),
                    username,
                    password,
                )
                .await
                {
                    Ok(token_info) => token_info,
                    Err(err) => bail!(format_login_error(&err)?),
                };

                login_connection = LoginConnection {
                    url,
                    danger_accept_invalid_certs: self.insecure,
                    token: token.token.unwrap_or_default(),
                    expiration: token.expiration.unwrap_or_default(),
                    bindle_url: self.bindle_server_url,
                    bindle_username: self.bindle_username,
                    bindle_password: self.bindle_password,
                };
            } else {
                // log in to the cloud API
                let connection_config = ConnectionConfig {
                    url,
                    insecure: self.insecure,
                    token: Default::default(),
                };

                if self.get_device_code {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &create_device_code(&Client::new(connection_config)).await?
                        )?
                    );
                    return Ok(());
                }

                let token: TokenInfo;
                if let Some(device_code) = self.check_device_code {
                    let client = Client::new(connection_config);
                    match client.login(device_code).await {
                        Ok(token_info) => {
                            if token_info.token.is_some() {
                                println!("{}", serde_json::to_string_pretty(&token_info)?);
                                token = token_info;
                            } else {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&json!({ "status": "waiting" }))?
                                );
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    };
                } else {
                    token = github_token(connection_config).await?;
                }

                login_connection = LoginConnection {
                    url: DEFAULT_CLOUD_URL.to_owned(),
                    danger_accept_invalid_certs: self.insecure,
                    token: token.token.unwrap_or_default(),
                    expiration: token.expiration.unwrap_or_default(),
                    bindle_url: None,
                    bindle_username: None,
                    bindle_password: None,
                };
            }
            std::fs::write(path, serde_json::to_string_pretty(&login_connection)?)?;
        } else {
            // log in to the default cloud API
            let connection_config = ConnectionConfig {
                url: DEFAULT_CLOUD_URL.to_owned(),
                insecure: self.insecure,
                token: Default::default(),
            };

            if self.get_device_code {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &create_device_code(&Client::new(connection_config)).await?
                    )?
                );
                return Ok(());
            }

            let token: TokenInfo;
            if let Some(device_code) = self.check_device_code {
                let client = Client::new(connection_config);
                match client.login(device_code).await {
                    Ok(token_info) => {
                        if token_info.token.is_some() {
                            println!("{}", serde_json::to_string_pretty(&token_info)?);
                            token = token_info;
                        } else {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&json!({ "status": "waiting" }))?
                            );
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        return Err(e);
                    }
                };
            } else {
                token = github_token(connection_config).await?;
            }

            let login_connection = LoginConnection {
                url: DEFAULT_CLOUD_URL.to_owned(),
                danger_accept_invalid_certs: self.insecure,
                token: token.token.unwrap_or_default(),
                expiration: token.expiration.unwrap_or_default(),
                bindle_url: None,
                bindle_username: None,
                bindle_password: None,
            };
            std::fs::write(path, serde_json::to_string_pretty(&login_connection)?)?;
        }

        Ok(())
    }
}

async fn github_token(
    connection_config: ConnectionConfig,
) -> Result<cloud_openapi::models::TokenInfo> {
    let client = Client::new(connection_config);

    // Generate a device code and a user code to activate it with
    let device_code = create_device_code(&client).await?;

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
                println!("Device authorized!");
                return Ok(response);
            }
            Err(_) => {
                println!("Waiting for device authorization...");
                tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                seconds_elapsed += POLL_INTERVAL_SECS;
            }
        };
    }
}

async fn create_device_code(client: &Client) -> Result<DeviceCodeItem> {
    client
        .create_device_code(Uuid::parse_str(SPIN_CLIENT_ID)?)
        .await
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
    let detail = match serde_json::from_str::<LoginHippoError>(err.to_string().as_str()) {
        Ok(e) => {
            if e.detail.ends_with(": ") {
                e.detail.replace(": ", ".")
            } else {
                e.detail
            }
        }
        Err(_) => err.to_string(),
    };
    Ok(format!("Problem logging into Hippo: {}", detail))
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

#[derive(PartialEq)]
enum AuthMethod {
    Github,
    UsernameAndPassword,
}

fn prompt_for_auth_method() -> AuthMethod {
    loop {
        // prompt the user for the authentication method
        print!("What authentication method does this server support?\n\n1. Sign in with GitHub\n2. Sign in with a username and password\n\nEnter a number: ");
        let mut input = String::new();
        stdin()
            .read_line(&mut input)
            .expect("unable to read user input");

        match input.trim() {
            "1" => {
                return AuthMethod::Github;
            }
            "2" => {
                return AuthMethod::UsernameAndPassword;
            }
            _ => {
                println!("invalid input. Please enter either 1 or 2.");
            }
        }
    }
}
