use std::{path::PathBuf, sync::Arc};

use anyhow::{bail, Context, Result};
use clap::Args;
use reqwest::Url;
use spin_loader::bindle::BindleConnectionInfo;
use spin_manifest::Application;

use crate::opts::*;

/// Options shared in common by all trigger implementations.
#[derive(Args, Debug)]
pub struct TriggerCommonOpts {
    /// Pass an environment variable (key=value) to all components of the application.
    #[clap(long = "env", short = 'e', parse(try_from_str = crate::parse_env_var))]
    pub env: Vec<(String, String)>,

    /// Log directory for the stdout and stderr of components.
    #[clap(
            name = APP_LOG_DIR,
            short = 'L',
            long = "log-dir",
            )]
    pub log: Option<PathBuf>,

    /// Disable Wasmtime cache.
    #[clap(
        name = DISABLE_WASMTIME_CACHE,
        long = "disable-cache",
        env = DISABLE_WASMTIME_CACHE,
        conflicts_with = WASMTIME_CACHE_FILE,
        takes_value = false,
    )]
    pub disable_cache: bool,

    /// Wasmtime cache configuration file.
    #[clap(
        name = WASMTIME_CACHE_FILE,
        long = "cache",
        env = WASMTIME_CACHE_FILE,
        conflicts_with = DISABLE_WASMTIME_CACHE,
    )]
    pub cache: Option<PathBuf>,

    /// Print output for given component(s) to stdout/stderr
    #[clap(
        name = FOLLOW_LOG_OPT,
        long = "follow",
        multiple_occurrences = true,
        )]
    pub follow_components: Vec<String>,

    /// Print all component output to stdout/stderr
    #[clap(
        long = "follow-all",
        conflicts_with = FOLLOW_LOG_OPT,
        )]
    pub follow_all_components: bool,
}

impl TriggerCommonOpts {
    pub fn wasmtime_config(&self) -> Result<wasmtime::Config> {
        let mut wasmtime_config = wasmtime::Config::default();
        if !self.disable_cache {
            match &self.cache {
                Some(p) => wasmtime_config.cache_config_load(p)?,
                None => wasmtime_config.cache_config_load_default()?,
            };
        }
        Ok(wasmtime_config)
    }

    pub async fn app_from_env(&self) -> Result<Application> {
        let working_dir = std::env::var("SPIN_WORKING_DIR").context("SPIN_WORKING_DIR")?;
        let manifest_url = std::env::var("SPIN_MANIFEST_URL").context("SPIN_MANIFEST_URL")?;

        let mut app = if let Some(manifest_file) = manifest_url.strip_prefix("file://") {
            let bindle_connection = std::env::var(BINDLE_URL_ENV)
                .ok()
                .map(|url| BindleConnectionInfo::new(url, false, None, None));
            spin_loader::from_file(manifest_file, working_dir, &bindle_connection).await?
        } else if let Some(bindle_url) = manifest_url.strip_prefix("bindle+") {
            let mut url: Url = bindle_url.parse()?;
            let bindle = url
                .query()
                .and_then(|s| s.strip_prefix("id="))
                .unwrap()
                .to_string();
            url.set_query(None);
            spin_loader::from_bindle(bindle.as_str(), url.as_str(), working_dir).await?
        } else {
            bail!("invalid SPIN_MANIFEST_URL {}", manifest_url);
        };

        crate::append_env(&mut app, &self.env)?;

        if let Some(ref mut resolver) = app.config_resolver {
            // TODO(lann): This should be safe but ideally this get_mut would be refactored away.
            let resolver = Arc::get_mut(resolver)
                .context("Internal error: app.config_resolver unexpectedly shared")?;
            // TODO(lann): Make config provider(s) configurable.
            resolver.add_provider(spin_config::provider::env::EnvProvider::default());
        }

        Ok(app)
    }
}
