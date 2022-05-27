use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Parser;
use spin_http_engine::{HttpTrigger, HttpTriggerExecutionConfig, TlsConfig};
use spin_trigger::{run_trigger, ExecutionOptions};

use super::trigger::TriggerCommonOpts;
use crate::opts::*;

/// Run the build command for each component.
#[derive(Parser, Debug)]
#[clap(about = "Run the HTTP trigger executor")]
pub struct TriggerHttpCommand {
    #[clap(flatten)]
    pub opts: TriggerCommonOpts,

    /// IP address and port to listen on
    #[clap(name = ADDRESS_OPT, long = "listen", default_value = "127.0.0.1:3000")]
    pub address: String,

    /// The path to the certificate to use for https, if this is not set, normal http will be used. The cert should be in PEM format
    #[clap(
            name = TLS_CERT_FILE_OPT,
            long = "tls-cert",
            env = TLS_CERT_ENV_VAR,
            requires = TLS_KEY_FILE_OPT,
        )]
    pub tls_cert: Option<PathBuf>,

    /// The path to the certificate key to use for https, if this is not set, normal http will be used. The key should be in PKCS#8 format
    #[clap(
            name = TLS_KEY_FILE_OPT,
            long = "tls-key",
            env = TLS_KEY_ENV_VAR,
            requires = TLS_CERT_FILE_OPT,
        )]
    pub tls_key: Option<PathBuf>,
}

impl TriggerHttpCommand {
    pub async fn run(&self) -> Result<()> {
        let app = self.opts.app_from_env().await?;

        let tls = match (self.tls_key.clone(), self.tls_cert.clone()) {
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

        run_trigger(
            app,
            ExecutionOptions::<HttpTrigger>::new(
                self.opts.kv_dir.clone(),
                self.opts.log.clone(),
                self.opts.follow_components(),
                HttpTriggerExecutionConfig::new(self.address.clone(), tls),
            ),
            Some(self.opts.wasmtime_config()?),
        )
        .await
    }
}
