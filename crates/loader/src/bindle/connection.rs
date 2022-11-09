use std::{path::PathBuf, sync::Arc};

use anyhow::{anyhow, Context, Result};
use bindle::{
    client::{
        tokens::{HttpBasic, LongLivedToken, NoToken, TokenManager},
        Client, ClientBuilder,
    },
    invoice::{
        signature::{KeyEntry, KeyRing},
        HealthResponse,
    },
};
use semver::{Version, VersionReq};
use tracing::log;

/// BindleConnectionInfo holds the details of a connection to a
/// Bindle server, including url, insecure configuration and an
/// auth token manager
#[derive(Clone)]
pub struct BindleConnectionInfo {
    base_url: String,
    allow_insecure: bool,
    token_manager: AnyAuth,
    keyring_path: PathBuf,
}

impl BindleConnectionInfo {
    /// Generates a new BindleConnectionInfo instance using the provided
    /// base_url, allow_insecure setting and optional username and password
    /// for basic http auth
    pub async fn new<I: Into<String>>(
        base_url: I,
        allow_insecure: bool,
        username: Option<String>,
        password: Option<String>,
        keyring_file: Option<PathBuf>,
    ) -> Result<Self> {
        let base_url: String = base_url.into();
        check_bindle_healthz(&base_url).await?;

        let token_manager: Box<dyn TokenManager + Send + Sync> = match (username, password) {
            (Some(u), Some(p)) => Box::new(HttpBasic::new(&u, &p)),
            _ => Box::new(NoToken::default()),
        };

        let keyring_path = match keyring_file {
            Some(dir) => dir,
            None => {
                let dir = ensure_config_dir().await?;
                dir.join("keyring.toml")
            }
        };

        Ok(Self {
            base_url,
            allow_insecure,
            token_manager: AnyAuth {
                token_manager: Arc::new(token_manager),
            },
            keyring_path,
        })
    }

    /// Generates a new BindleConnectionInfo instance using the provided
    /// base_url, allow_insecure setting and token.
    pub async fn from_token<I: Into<String>>(
        base_url: I,
        allow_insecure: bool,
        token: I,
        keyring_file: Option<PathBuf>,
    ) -> Result<Self> {
        let base_url: String = base_url.into();
        check_bindle_healthz(&base_url).await?;

        let token_manager: Box<dyn TokenManager + Send + Sync> =
            Box::new(LongLivedToken::new(&token.into()));

        let keyring_path = match keyring_file {
            Some(dir) => dir,
            None => {
                let dir = ensure_config_dir().await?;
                dir.join("keyring.toml")
            }
        };

        Ok(Self {
            base_url,
            allow_insecure,
            token_manager: AnyAuth {
                token_manager: Arc::new(token_manager),
            },
            keyring_path,
        })
    }

    /// Returns a client based on this instance's configuration
    pub async fn client(&self) -> bindle::client::Result<Client<AnyAuth>> {
        let mut keyring = read_bindle_keyring(&self.keyring_path)
            .await
            .unwrap_or_else(|e| {
                log::error!(
                    "can't read bindle keyring file {:?}, err: {:?}",
                    &self.keyring_path,
                    e
                );
                KeyRing::default()
            });

        let tmp_client = ClientBuilder::default()
            .http2_prior_knowledge(false)
            .danger_accept_invalid_certs(self.allow_insecure)
            .build(
                &self.base_url,
                self.token_manager.clone(),
                Arc::new(keyring.clone()),
            )?;

        log::trace!("Fetching host keys from bindle server");
        let host_keys = tmp_client.get_host_keys().await?;
        let filtered_keys: Vec<KeyEntry> = host_keys
            .key
            .into_iter()
            .filter(|k| !keyring.key.iter().any(|current| current.key == k.key))
            .collect();
        keyring.key.extend(filtered_keys);
        log::info!("keyring: {:?}", &keyring);

        ClientBuilder::default()
            .http2_prior_knowledge(false)
            .danger_accept_invalid_certs(self.allow_insecure)
            .build(
                &self.base_url,
                self.token_manager.clone(),
                Arc::new(keyring),
            )
    }

    /// Returns the base url for this client.
    pub fn base_url(&self) -> &str {
        self.base_url.as_ref()
    }
}

/// AnyAuth wraps an authentication token manager which applies
/// the appropriate auth header per its configuration
#[derive(Clone)]
pub struct AnyAuth {
    token_manager: Arc<Box<dyn TokenManager + Send + Sync>>,
}

#[async_trait::async_trait]
impl TokenManager for AnyAuth {
    async fn apply_auth_header(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> bindle::client::Result<reqwest::RequestBuilder> {
        self.token_manager.apply_auth_header(builder).await
    }
}

async fn read_bindle_keyring(keyring_path: &PathBuf) -> bindle::client::Result<KeyRing> {
    let raw_data = tokio::fs::read(keyring_path).await?;
    let res: KeyRing = toml::from_slice(&raw_data)?;
    Ok(res)
}

async fn ensure_config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .map(|v| v.join("bindle/"))
        .unwrap_or_else(|| "./bindle".into());
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir)
}

async fn check_bindle_healthz(url: &str) -> Result<()> {
    let base_url = url::Url::parse(url)?;
    let healthz_url = base_url.join("/healthz")?;
    let result = reqwest::get(healthz_url.to_string())
        .await?
        .error_for_status()
        .with_context(|| format!("Bindle server {} is unhealthy", base_url))?
        .json::<HealthResponse>()
        .await.with_context(|| "Can't parse bindle server /healthz response as json, please run bindle-server with version >=0.9.0-rc.1")?;

    let server_version = Version::parse(&result.version)
        .with_context(|| format!("can't parse version {}", &result.version))?;
    let min_version_req = VersionReq::parse(">=0.9.0-rc.1")?;
    if !min_version_req.matches(&server_version) {
        return Err(anyhow!(
            "bindle-server version is {}, please run bindle-server with version >= 0.9.0-rc.1",
            &result.version
        ));
    }
    Ok(())
}
