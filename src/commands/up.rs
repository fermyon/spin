use crate::opts::*;
use anyhow::{bail, Context, Result};
use spin_engine::{Builder, ExecutionContextConfiguration};
use spin_http_engine::{HttpTrigger, TlsConfig};
use spin_manifest::{Application, ApplicationTrigger};
use spin_redis_engine::RedisTrigger;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use structopt::{clap::AppSettings, StructOpt};
use tempfile::TempDir;

/// Start the Fermyon runtime.
#[derive(StructOpt, Debug)]
#[structopt(
    about = "Start the Spin application",
    global_settings = &[AppSettings::ColoredHelp, AppSettings::ArgRequiredElseHelp]
)]

pub struct UpCommand {
    #[structopt(flatten)]
    pub opts: UpOpts,

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
}

#[derive(StructOpt, Debug)]
pub struct UpOpts {
    /// IP address and port to listen on
    #[structopt(name = ADDRESS_OPT, long = "listen", default_value = "127.0.0.1:3000")]
    pub address: String,
    /// Temporary directory for the static assets of the components.
    #[structopt(long = "temp")]
    pub tmp: Option<PathBuf>,
    /// Pass an environment variable (key=value) to all components of the application.
    #[structopt(long = "env", short = "e", parse(try_from_str = crate::parse_env_var))]
    pub env: Vec<(String, String)>,

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

    /// Disable Wasmtime cache.
    #[structopt(
        name = DISABLE_WASMTIME_CACHE,
        long = "disable-cache",
        env = DISABLE_WASMTIME_CACHE,
        conflicts_with = WASMTIME_CACHE_FILE,
        takes_value = false,
    )]
    pub disable_cache: bool,

    /// Wasmtime cache configuration file.
    #[structopt(
        name = WASMTIME_CACHE_FILE,
        long = "cache",
        env = WASMTIME_CACHE_FILE,
        conflicts_with = DISABLE_WASMTIME_CACHE,
    )]
    pub cache: Option<PathBuf>,
}

impl UpCommand {
    pub async fn run(self) -> Result<()> {
        let working_dir_holder = match &self.opts.tmp {
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
        crate::append_env(&mut app, &self.opts.env)?;

        if let Some(ref mut resolver) = app.config_resolver {
            // TODO(lann): This should be safe but ideally this get_mut would be refactored away.
            let resolver = Arc::get_mut(resolver)
                .context("Internal error: app.config_resolver unexpectedly shared")?;
            // TODO(lann): Make config provider(s) configurable.
            resolver.add_provider(spin_config::provider::env::EnvProvider::default());
        }

        let tls = match (self.opts.tls_key.clone(), self.opts.tls_cert.clone()) {
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
                let builder = self.prepare_ctx_builder(app.clone()).await?;
                let trigger = HttpTrigger::new(builder, app, self.opts.address, tls).await?;
                trigger.run().await?;
            }
            ApplicationTrigger::Redis(_) => {
                let builder = self.prepare_ctx_builder(app.clone()).await?;
                let trigger = RedisTrigger::new(builder, app).await?;
                trigger.run().await?;
            }
        }

        // We need to be absolutely sure it stays alive until this point: we don't want
        // any temp directory to be deleted prematurely.
        drop(working_dir_holder);

        Ok(())
    }

    async fn prepare_ctx_builder<T: Default + 'static>(
        &self,
        app: Application,
    ) -> Result<Builder<T>> {
        let config = ExecutionContextConfiguration {
            log_dir: self.opts.log.clone(),
            ..app.into()
        };

        let mut wasmtime_config = wasmtime::Config::default();
        if !self.opts.disable_cache {
            match &self.opts.cache {
                Some(p) => wasmtime_config.cache_config_load(p)?,
                None => wasmtime_config.cache_config_load_default()?,
            };
        }

        let mut builder = Builder::with_wasmtime_config(config, wasmtime_config)?;
        builder.link_defaults()?;
        builder.add_host_component(wasi_outbound_http::OutboundHttpComponent)?;
        builder.add_host_component(outbound_redis::OutboundRedis)?;
        Ok(builder)
    }
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
