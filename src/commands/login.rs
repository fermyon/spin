use std::io::{stdin, Write};
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
    BINDLE_PASSWORD, BINDLE_SERVER_URL_OPT, BINDLE_URL_ENV, BINDLE_USERNAME,
    DEPLOYMENT_ENV_NAME_ENV, HIPPO_PASSWORD, HIPPO_SERVER_URL_OPT, HIPPO_URL_ENV, HIPPO_USERNAME,
    INSECURE_OPT,
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
        long = "url",
        env = HIPPO_URL_ENV,
    )]
    pub hippo_server_url: Option<String>,

    /// Hippo username
    #[clap(
        name = HIPPO_USERNAME,
        long = "username",
        env = HIPPO_USERNAME,
        requires = HIPPO_PASSWORD,
    )]
    pub hippo_username: Option<String>,

    /// Hippo password
    #[clap(
        name = HIPPO_PASSWORD,
        long = "password",
        env = HIPPO_PASSWORD,
        requires = HIPPO_USERNAME,
    )]
    pub hippo_password: Option<String>,

    /// Display login status
    #[clap(
        name = "status",
        long = "status",
        takes_value = false,
        conflicts_with = "list",
        conflicts_with = "get-device-code",
        conflicts_with = "check-device-code"
    )]
    pub status: bool,

    // fetch a device code
    #[clap(
        name = "get-device-code",
        long = "get-device-code",
        takes_value = false,
        hide = true,
        conflicts_with = "status",
        conflicts_with = "check-device-code"
    )]
    pub get_device_code: bool,

    // check a device code
    #[clap(
        name = "check-device-code",
        long = "check-device-code",
        hide = true,
        conflicts_with = "status",
        conflicts_with = "get-device-code"
    )]
    pub check_device_code: Option<String>,

    // authentication method used for logging in (username|github)
    #[clap(
        name = "auth-method",
        long = "auth-method",
        env = "AUTH_METHOD",
        arg_enum
    )]
    pub method: Option<AuthMethod>,

    /// Save the login details under the specified name instead of making them
    /// the default. Use named environments with `spin deploy --environment-name <name>`.
    #[clap(
        name = "environment-name",
        long = "environment-name",
        env = DEPLOYMENT_ENV_NAME_ENV
    )]
    pub deployment_env_id: Option<String>,

    /// List saved logins.
    #[clap(
        name = "list",
        long = "list",
        takes_value = false,
        conflicts_with = "environment-name",
        conflicts_with = "status",
        conflicts_with = "get-device-code",
        conflicts_with = "check-device-code"
    )]
    pub list: bool,
}

impl LoginCommand {
    pub async fn run(&self) -> Result<()> {
        match (
            self.list,
            self.status,
            self.get_device_code,
            &self.check_device_code,
        ) {
            (true, false, false, None) => self.run_list().await,
            (false, true, false, None) => self.run_status().await,
            (false, false, true, None) => self.run_get_device_code().await,
            (false, false, false, Some(device_code)) => {
                self.run_check_device_code(device_code).await
            }
            (false, false, false, None) => self.run_interactive_login().await,
            _ => Err(anyhow::anyhow!("Invalid combination of options")), // Should never happen
        }
    }

    async fn run_list(&self) -> Result<()> {
        let root = config_root_dir()?;

        ensure(&root)?;

        let json_file_stems = std::fs::read_dir(&root)
            .with_context(|| format!("Failed to read config directory {}", root.display()))?
            .filter_map(environment_name_from_path)
            .collect::<Vec<_>>();

        for s in json_file_stems {
            println!("{}", s);
        }

        Ok(())
    }

    async fn run_status(&self) -> Result<()> {
        let path = self.config_file_path()?;
        let data = fs::read_to_string(&path)
            .await
            .context("Cannnot display login information")?;
        println!("{}", data);
        Ok(())
    }

    async fn run_get_device_code(&self) -> Result<()> {
        let connection_config = self.anon_connection_config();
        let device_code_info = create_device_code(&Client::new(connection_config)).await?;

        println!("{}", serde_json::to_string_pretty(&device_code_info)?);

        Ok(())
    }

    async fn run_check_device_code(&self, device_code: &str) -> Result<()> {
        let connection_config = self.anon_connection_config();
        let client = Client::new(connection_config);
        let token_info = client.login(device_code.to_owned()).await?;

        let token_readiness = if token_info.token.is_some() {
            TokenReadiness::Ready(token_info)
        } else {
            TokenReadiness::Unready
        };

        match token_readiness {
            TokenReadiness::Ready(token_info) => {
                println!("{}", serde_json::to_string_pretty(&token_info)?);
                let login_connection = self.login_connection_for_token(token_info);
                self.save_login_info(&login_connection)?;
            }
            TokenReadiness::Unready => {
                let waiting = json!({ "status": "waiting" });
                println!("{}", serde_json::to_string_pretty(&waiting)?);
            }
        }

        Ok(())
    }

    async fn run_interactive_login(&self) -> Result<()> {
        let login_connection = match self.auth_method() {
            AuthMethod::Github => self.run_interactive_gh_login().await?,
            AuthMethod::UsernameAndPassword => self.run_interactive_basic_login().await?,
        };
        self.save_login_info(&login_connection)
    }

    async fn run_interactive_gh_login(&self) -> Result<LoginConnection> {
        // log in to the cloud API
        let connection_config = self.anon_connection_config();
        let token_info = github_token(connection_config).await?;

        Ok(self.login_connection_for_token(token_info))
    }

    async fn run_interactive_basic_login(&self) -> Result<LoginConnection> {
        let username = prompt_if_not_provided(&self.hippo_username, "Hippo username")?;
        let password = match &self.hippo_password {
            Some(password) => password.to_owned(),
            None => {
                print!("Hippo password: ");
                std::io::stdout().flush()?;
                rpassword::read_password()
                    .expect("unable to read user input")
                    .trim()
                    .to_owned()
            }
        };

        let bindle_url = prompt_if_not_provided(&self.bindle_server_url, "Bindle URL")?;

        // If Bindle URL was provided and Bindle username and password were not, assume Bindle
        // is unauthenticated.  If Bindle URL was prompted for, or Bindle username or password
        // is provided, ask the user.
        let mut bindle_username = self.bindle_username.clone();
        let mut bindle_password = self.bindle_password.clone();

        let unauthenticated_bindle_server_provided = self.bindle_server_url.is_some()
            && self.bindle_username.is_none()
            && self.bindle_password.is_none();
        if !unauthenticated_bindle_server_provided {
            let bindle_username_text = prompt_if_not_provided(
                &self.bindle_username,
                "Bindle username (blank for unauthenticated)",
            )?;
            bindle_username = if bindle_username_text.is_empty() {
                None
            } else {
                Some(bindle_username_text)
            };
            bindle_password = match bindle_username {
                None => None,
                Some(_) => Some(prompt_if_not_provided(
                    &self.bindle_password,
                    "Bindle password",
                )?),
            };
        }

        // log in with username/password
        let token = match HippoClient::login(
            &HippoClient::new(ConnectionInfo {
                url: self.url().to_owned(),
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

        Ok(LoginConnection {
            url: self.url().to_owned(),
            danger_accept_invalid_certs: self.insecure,
            token: token.token.unwrap_or_default(),
            expiration: token.expiration.unwrap_or_default(),
            bindle_url: Some(bindle_url),
            bindle_username,
            bindle_password,
        })
    }

    fn login_connection_for_token(&self, token_info: TokenInfo) -> LoginConnection {
        let login_connection = LoginConnection {
            url: self.url().to_owned(),
            danger_accept_invalid_certs: self.insecure,
            token: token_info.token.unwrap_or_default(),
            expiration: token_info.expiration.unwrap_or_default(),
            bindle_url: None,
            bindle_username: None,
            bindle_password: None,
        };
        login_connection
    }

    fn config_file_path(&self) -> Result<PathBuf> {
        let root = config_root_dir()?;

        ensure(&root)?;

        let file_stem = match &self.deployment_env_id {
            None => "config",
            Some(id) => id,
        };
        let file = format!("{}.json", file_stem);

        let path = root.join(file);

        Ok(path)
    }

    fn anon_connection_config(&self) -> ConnectionConfig {
        ConnectionConfig {
            url: self.url().to_owned(),
            insecure: self.insecure,
            token: Default::default(),
        }
    }

    fn url(&self) -> &str {
        if let Some(u) = &self.hippo_server_url {
            u
        } else {
            DEFAULT_CLOUD_URL
        }
    }

    fn auth_method(&self) -> AuthMethod {
        if let Some(method) = &self.method {
            method.clone()
        } else if self.get_device_code || self.check_device_code.is_some() {
            AuthMethod::Github
        } else if self.hippo_username.is_some() || self.hippo_password.is_some() {
            AuthMethod::UsernameAndPassword
        } else if self.hippo_server_url.is_some() {
            // prompt the user for the authentication method
            // TODO: implement a server "feature" check that tells us what authentication methods it supports
            prompt_for_auth_method()
        } else {
            AuthMethod::Github
        }
    }

    fn save_login_info(&self, login_connection: &LoginConnection) -> Result<(), anyhow::Error> {
        let path = self.config_file_path()?;
        std::fs::write(path, serde_json::to_string_pretty(login_connection)?)?;
        Ok(())
    }
}

fn config_root_dir() -> Result<PathBuf, anyhow::Error> {
    let root = dirs::config_dir()
        .context("Cannot find configuration directory")?
        .join("fermyon");
    Ok(root)
}

fn prompt_if_not_provided(provided: &Option<String>, prompt_text: &str) -> Result<String> {
    match provided {
        Some(value) => Ok(value.to_owned()),
        None => {
            print!("{}: ", prompt_text);
            std::io::stdout().flush()?;
            let mut input = String::new();
            stdin()
                .read_line(&mut input)
                .expect("unable to read user input");
            Ok(input.trim().to_owned())
        }
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
                if response.token.is_none() {
                    println!("Waiting for device authorization...");
                    tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                    seconds_elapsed += POLL_INTERVAL_SECS;
                } else {
                    println!("Device authorized!");
                    return Ok(response);
                }
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

/// The method by which to authenticate the login.
#[derive(clap::ArgEnum, Clone, Debug, Eq, PartialEq)]
pub enum AuthMethod {
    #[clap(name = "github")]
    Github,
    #[clap(name = "username")]
    UsernameAndPassword,
}

fn prompt_for_auth_method() -> AuthMethod {
    loop {
        // prompt the user for the authentication method
        print!("What authentication method does this server support?\n\n1. Sign in with GitHub\n2. Sign in with a username and password\n\nEnter a number: ");
        std::io::stdout().flush().unwrap();
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

enum TokenReadiness {
    Ready(TokenInfo),
    Unready,
}

fn environment_name_from_path(dir_entry: std::io::Result<std::fs::DirEntry>) -> Option<String> {
    let json_ext = std::ffi::OsString::from("json");
    let default_name = "(default)";
    match dir_entry {
        Err(_) => None,
        Ok(de) => {
            if is_file_with_extension(&de, &json_ext) {
                de.path().file_stem().map(|stem| {
                    let s = stem.to_string_lossy().to_string();
                    if s == "config" {
                        default_name.to_owned()
                    } else {
                        s
                    }
                })
            } else {
                None
            }
        }
    }
}

fn is_file_with_extension(de: &std::fs::DirEntry, extension: &std::ffi::OsString) -> bool {
    match de.file_type() {
        Err(_) => false,
        Ok(t) => {
            if t.is_file() {
                de.path().extension() == Some(extension)
            } else {
                false
            }
        }
    }
}
