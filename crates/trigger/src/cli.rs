use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::{Args, IntoApp, Parser};
use serde::de::DeserializeOwned;

use crate::stdio::StdioLoggingTriggerHooks;
use crate::{config::TriggerExecutorBuilderConfig, loader::TriggerLoader, stdio::FollowComponents};
use crate::{TriggerExecutor, TriggerExecutorBuilder};

pub const APP_LOG_DIR: &str = "APP_LOG_DIR";
pub const FOLLOW_LOG_OPT: &str = "FOLLOW_ID";
pub const RUNTIME_CONFIG_FILE: &str = "RUNTIME_CONFIG_FILE";

// Set by `spin up`
pub const SPIN_LOCKED_URL: &str = "SPIN_LOCKED_URL";
pub const SPIN_WORKING_DIR: &str = "SPIN_WORKING_DIR";

/// A command that runs a TriggerExecutor.
#[derive(Parser, Debug)]
#[clap(next_help_heading = "TRIGGER OPTIONS")]
pub struct TriggerExecutorCommand<Executor: TriggerExecutor>
where
    Executor::RunConfig: Args,
{
    /// Log directory for the stdout and stderr of components.
    #[clap(
            name = APP_LOG_DIR,
            short = 'L',
            long = "log-dir",
            )]
    pub log: Option<PathBuf>,

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

    /// Set the static assets of the components in the temporary directory as writable.
    #[clap(long = "allow-transient-write")]
    pub allow_transient_write: bool,

    /// Configuration file for config providers and wasmtime config.
    #[clap(
        name = RUNTIME_CONFIG_FILE,
        long = "runtime-config-file",
        env = RUNTIME_CONFIG_FILE,
    )]
    pub runtime_config_file: Option<PathBuf>,

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
    Executor::TriggerConfig: DeserializeOwned,
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

        // Required env vars
        let working_dir = std::env::var(SPIN_WORKING_DIR).context(SPIN_WORKING_DIR)?;
        let locked_url = std::env::var(SPIN_LOCKED_URL).context(SPIN_LOCKED_URL)?;

        let loader = TriggerLoader::new(&working_dir, self.allow_transient_write);

        let base_runtime_config_dir = match &self.runtime_config_file {
            Some(p) => p.parent().ok_or_else(|| {
                anyhow!(
                    "Failed to get containing directory for runtime config file '{}'",
                    &p.display()
                )
            })?,
            None => Path::new(&working_dir),
        };
        let trigger_config =
            TriggerExecutorBuilderConfig::load_from_file(self.runtime_config_file.clone())?;

        let executor: Executor = {
            let mut builder = TriggerExecutorBuilder::new(loader);
            self.update_wasmtime_config(
                builder.wasmtime_config_mut(),
                &trigger_config,
                base_runtime_config_dir,
            )?;

            let logging_hooks = StdioLoggingTriggerHooks::new(self.follow_components(), self.log);
            builder.hooks(logging_hooks);

            builder.build(locked_url, &trigger_config).await?
        };

        let run_fut = executor.run(self.run_config);

        let (abortable, abort_handle) = futures::future::abortable(run_fut);
        ctrlc::set_handler(move || abort_handle.abort())?;
        match abortable.await {
            Ok(Ok(())) => {
                tracing::info!("Trigger executor shut down: exiting");
                Ok(())
            }
            Ok(Err(err)) => {
                tracing::error!("Trigger executor failed");
                Err(err)
            }
            Err(_aborted) => {
                tracing::info!("User requested shutdown: exiting");
                Ok(())
            }
        }
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

    fn update_wasmtime_config(
        &self,
        wasmtime_config: &mut spin_core::wasmtime::Config,
        trigger_config: &TriggerExecutorBuilderConfig,
        base_dir: impl AsRef<Path>,
    ) -> Result<()> {
        if let Some(p) = &trigger_config.wasmtime_config.cache_file {
            let p = match Path::new(p).is_absolute() {
                true => PathBuf::from(p),
                false => base_dir.as_ref().join(p),
            };
            wasmtime_config.cache_config_load(p)?;
        }

        let wasm_backtrace_details = match &*trigger_config.wasmtime_config.wasm_backtrace_details {
            "Enable" => wasmtime::WasmBacktraceDetails::Enable,
            "Environment" => wasmtime::WasmBacktraceDetails::Environment,
            _ => wasmtime::WasmBacktraceDetails::Disable,
        };
        tracing::trace!("wasm_backtrace_details {:?}", wasm_backtrace_details);
        wasmtime_config.wasm_backtrace_details(wasm_backtrace_details);
        Ok(())
    }
}
