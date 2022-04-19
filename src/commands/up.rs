use anyhow::{bail, Result};
use spin_engine::{Builder, ExecutionContext, ExecutionContextConfiguration};
use spin_http_engine::{spin_http::SpinHttpData, HttpTrigger, TlsConfig};
use spin_manifest::{Application, ApplicationTrigger, CoreComponent};
use spin_redis_engine::RedisTrigger;
use std::path::{Path, PathBuf};
use structopt::{clap::AppSettings, StructOpt};
use tempfile::TempDir;

pub const DEFAULT_MANIFEST_FILE: &str = "spin.toml";

const APP_CONFIG_FILE_OPT: &str = "APP_CONFIG_FILE";
const BINDLE_ID_OPT: &str = "BINDLE_ID";
const BINDLE_SERVER_URL_OPT: &str = "BINDLE_SERVER_URL";
const BINDLE_URL_ENV: &str = "BINDLE_URL";
const APP_LOG_DIR: &str = "APP_LOG_DIR";

const TLS_CERT_FILE_OPT: &str = "TLS_CERT_FILE";
const TLS_KEY_FILE_OPT: &str = "TLS_KEY_FILE";

const TLS_CERT_ENV_VAR: &str = "SPIN_TLS_CERT";
const TLS_KEY_ENV_VAR: &str = "SPIN_TLS_KEY";

/// Start the Fermyon runtime.
#[derive(StructOpt, Debug)]
#[structopt(
    about = "Start the Spin application",
    global_settings = &[AppSettings::ColoredHelp, AppSettings::ArgRequiredElseHelp]
)]

pub struct UpCommand {
    /// IP address and port to listen on
    #[structopt(long = "listen", default_value = "127.0.0.1:3000")]
    pub address: String,
    /// Path to spin.toml.
    #[structopt(
        name = APP_CONFIG_FILE_OPT,
        short = "f",
        long = "file",
        conflicts_with = BINDLE_ID_OPT,
    )]
    pub app: Option<PathBuf>,
    /// ID of application bindle.
    #[structopt(
        name = BINDLE_ID_OPT,
        short = "b",
        long = "bindle",
        conflicts_with = APP_CONFIG_FILE_OPT,
        requires = BINDLE_SERVER_URL_OPT,
    )]
    pub bindle: Option<String>,
    /// URL of bindle server.
    #[structopt(
        name = BINDLE_SERVER_URL_OPT,
        long = "server",
        env = BINDLE_URL_ENV,
    )]
    pub server: Option<String>,
    /// Temporary directory for the static assets of the components.
    #[structopt(long = "temp")]
    pub tmp: Option<PathBuf>,
    /// Pass an environment variable (key=value) to all components of the application.
    #[structopt(long = "env", short = "e", parse(try_from_str = parse_env_var))]
    env: Vec<(String, String)>,

    /// The path to the certificate to use for https, if this is not set, normal http will be used. The cert should be in PEM format
    #[structopt(
        name = TLS_CERT_FILE_OPT,
        long = "tls-cert",
        env = TLS_CERT_ENV_VAR,
        requires = TLS_KEY_FILE_OPT,
    )]
    pub tls_cert: Option<PathBuf>,

    /// The path to the certificate key to use for https, if this is not set, normal http will be used. The key should be in PKCS#8 format
    #[structopt(
        name = TLS_KEY_FILE_OPT,
        long = "tls-key",
        env = TLS_KEY_ENV_VAR,
        requires = TLS_CERT_FILE_OPT,
    )]
    pub tls_key: Option<PathBuf>,
    /// Log directory for the stdout and stderr of components.
    #[structopt(
        name = APP_LOG_DIR,
        short = "L",
        long = "log-dir",
        )]
    pub log: Option<PathBuf>,
}

impl UpCommand {
    pub async fn run(self) -> Result<()> {
        let working_dir_holder = match &self.tmp {
            None => WorkingDirectory::Temporary(tempfile::tempdir()?),
            Some(d) => WorkingDirectory::Given(d.to_owned()),
        };
        let working_dir = working_dir_holder.path();

        let mut app = match (&self.app, &self.bindle) {
            (app, None) => {
                let manifest_file = app
                    .as_deref()
                    .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());
                spin_loader::from_file(manifest_file, working_dir).await?
            }
            (None, Some(bindle)) => match &self.server {
                Some(server) => spin_loader::from_bindle(bindle, server, working_dir).await?,
                _ => bail!("Loading from a bindle requires a Bindle server URL"),
            },
            (Some(_), Some(_)) => bail!("Specify only one of app file or bindle ID"),
        };
        append_env(&mut app, &self.env)?;

        // if let Some(ref mut resolver) = app.config_resolver {
        //     // TODO(lann): Make config provider(s) configurable.
        //     resolver.add_provider(spin_config::provider::env::EnvProvider::default());
        // }

        let tls = match (self.tls_key, self.tls_cert) {
            (Some(key_path), Some(cert_path)) => {
                if !cert_path.is_file() {
                    bail!("TLS certificate file does not exist or is not a file")
                }
                if !key_path.is_file() {
                    bail!("TLS key file does not exist or is not a file")
                }
                Some(TlsConfig {
                    cert_path,
                    key_path,
                })
            }
            (None, None) => None,
            _ => unreachable!(),
        };

        match &app.info.trigger {
            ApplicationTrigger::Http(_) => {
                let trigger = HttpTrigger::new(self.address, app, tls, self.log, None).await?;
                trigger.run().await?;
            }
            ApplicationTrigger::Redis(_) => {
                let trigger = RedisTrigger::new(app, self.log, None).await?;
                trigger.run().await?;
            }
        }

        // We need to be absolutely sure it stays alive until this point: we don't want
        // any temp directory to be deleted prematurely.
        drop(working_dir_holder);

        Ok(())
    }
}

/// Parse the environment variables passed in `key=value` pairs.
fn parse_env_var(s: &str) -> Result<(String, String)> {
    let parts: Vec<_> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("Environment variable must be of the form `key=value`");
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
}

/// Append the environment variables passed as options to all components.
fn append_env(app: &mut Application<CoreComponent>, env: &[(String, String)]) -> Result<()> {
    for c in app.components.iter_mut() {
        for (k, v) in env {
            c.wasm.environment.insert(k.clone(), v.clone());
        }
    }
    Ok(())
}

enum WorkingDirectory {
    Given(PathBuf),
    Temporary(TempDir),
}

impl WorkingDirectory {
    fn path(&self) -> &Path {
        match self {
            Self::Given(p) => p,
            Self::Temporary(t) => t.path(),
        }
    }
}
