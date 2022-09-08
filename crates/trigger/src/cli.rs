use std::{error::Error, path::PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, IntoApp, Parser};
use spin_engine::io::FollowComponents;
use spin_loader::bindle::BindleConnectionInfo;
use spin_manifest::{Application, ApplicationTrigger, TriggerConfig};

use crate::{TriggerExecutor, TriggerExecutorBuilder};

pub const APP_LOG_DIR: &str = "APP_LOG_DIR";
pub const DISABLE_WASMTIME_CACHE: &str = "DISABLE_WASMTIME_CACHE";
pub const FOLLOW_LOG_OPT: &str = "FOLLOW_ID";
pub const WASMTIME_CACHE_FILE: &str = "WASMTIME_CACHE_FILE";

/// A command that runs a TriggerExecutor.
#[derive(Parser, Debug)]
#[clap(next_help_heading = "TRIGGER OPTIONS")]
pub struct TriggerExecutorCommand<Executor: TriggerExecutor>
where
    Executor::RunConfig: Args,
{
    /// Pass an environment variable (key=value) to all components of the application.
    #[clap(long = "env", short = 'e', parse(try_from_str = parse_env_var))]
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

    #[clap(flatten)]
    pub run_config: Executor::RunConfig,

    #[clap(long = "help-args-only", hide = true)]
    pub help_args_only: bool,
}

/// An empty implementation of clap::Args to be used as TriggerExecutor::RunConfig
/// for executors that do not need additional CLI args.
#[derive(Args)]
pub struct NoArgs;

impl<Executor: TriggerExecutor> TriggerExecutorCommand<Executor>
where
    Executor::RunConfig: Args,
    Executor::GlobalConfig: TryFrom<ApplicationTrigger>,
    <Executor::GlobalConfig as TryFrom<ApplicationTrigger>>::Error: Error + Send + Sync + 'static,
    Executor::TriggerConfig: TryFrom<(String, TriggerConfig)>,
    <Executor::TriggerConfig as TryFrom<(String, TriggerConfig)>>::Error:
        Error + Send + Sync + 'static,
{
    /// Create a new TriggerExecutorBuilder from this TriggerExecutorCommand.
    pub async fn run(self) -> Result<()> {
        if self.help_args_only {
            Self::command()
                .disable_help_flag(true)
                .help_template("{all-args}")
                .print_long_help()?;
            return Ok(());
        }

        let app = self.build_application().await?;
        let mut builder = TriggerExecutorBuilder::new(app);
        self.update_wasmtime_config(builder.wasmtime_config_mut())?;
        builder.follow_components(self.follow_components());
        if let Some(log_dir) = self.log {
            builder.log_dir(log_dir);
        }

        let executor: Executor = builder.build().await?;
        let run_fut = executor.run(self.run_config);

        let (abortable, abort_handle) = futures::future::abortable(run_fut);
        ctrlc::set_handler(move || abort_handle.abort())?;
        match abortable.await {
            Ok(Ok(())) => {
                tracing::info!("Trigger executor shut down: exiting");
                Ok(())
            }
            Ok(Err(err)) => {
                tracing::error!("Trigger executor failed: {:?}", err);
                Err(err)
            }
            Err(_aborted) => {
                tracing::info!("User requested shutdown: exiting");
                Ok(())
            }
        }
    }
}

impl<Executor: TriggerExecutor> TriggerExecutorCommand<Executor>
where
    Executor::RunConfig: Args,
{
    pub async fn build_application(&self) -> Result<Application> {
        let working_dir = std::env::var("SPIN_WORKING_DIR").context("SPIN_WORKING_DIR")?;
        let manifest_url = std::env::var("SPIN_MANIFEST_URL").context("SPIN_MANIFEST_URL")?;
        let allow_transient_write: bool = std::env::var("SPIN_ALLOW_TRANSIENT_WRITE")
            .unwrap_or_else(|_| "false".to_string())
            .trim()
            .parse()
            .context("SPIN_ALLOW_TRANSIENT_WRITE")?;

        // TODO(lann): Find a better home for this; spin_loader?
        let mut app = if let Some(manifest_file) = manifest_url.strip_prefix("file://") {
            let bindle_connection = std::env::var("BINDLE_URL")
                .ok()
                .map(|url| BindleConnectionInfo::new(url, false, None, None));
            spin_loader::from_file(
                manifest_file,
                working_dir,
                &bindle_connection,
                allow_transient_write,
            )
            .await?
        } else if let Some(bindle_url) = manifest_url.strip_prefix("bindle+") {
            let (bindle_server, bindle_id) = bindle_url
                .rsplit_once("?id=")
                .context("invalid bindle URL")?;
            spin_loader::from_bindle(bindle_id, bindle_server, working_dir, allow_transient_write)
                .await?
        } else {
            bail!("invalid SPIN_MANIFEST_URL {}", manifest_url);
        };

        // Apply --env to all components in the given app
        for c in app.components.iter_mut() {
            for (k, v) in self.env.iter().cloned() {
                c.wasm.environment.insert(k, v);
            }
        }

        Ok(app)
    }

    pub fn follow_components(&self) -> FollowComponents {
        if self.follow_all_components {
            FollowComponents::All
        } else if self.follow_components.is_empty() {
            FollowComponents::None
        } else {
            let followed = self.follow_components.clone().into_iter().collect();
            FollowComponents::Named(followed)
        }
    }

    fn update_wasmtime_config(&self, config: &mut wasmtime::Config) -> Result<()> {
        // Apply --cache / --disable-cache
        if !self.disable_cache {
            match &self.cache {
                Some(p) => config.cache_config_load(p)?,
                None => config.cache_config_load_default()?,
            };
        }
        Ok(())
    }
}

// Parse the environment variables passed in `key=value` pairs.
fn parse_env_var(s: &str) -> Result<(String, String)> {
    let parts: Vec<_> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("Environment variable must be of the form `key=value`");
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
}
