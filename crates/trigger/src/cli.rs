use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, IntoApp, Parser};
use serde::de::DeserializeOwned;
use spin_app::Loader;
use spin_common::{arg_parser::parse_kv, sloth};

use crate::network::Network;
use crate::runtime_config::llm::LLmOptions;
use crate::runtime_config::sqlite::SqlitePersistenceMessageHook;
use crate::runtime_config::SummariseRuntimeConfigHook;
use crate::stdio::StdioLoggingTriggerHooks;
use crate::{
    loader::TriggerLoader,
    runtime_config::{key_value::KeyValuePersistenceMessageHook, RuntimeConfig},
    stdio::FollowComponents,
};
use crate::{TriggerExecutor, TriggerExecutorBuilder};

mod launch_metadata;
pub use launch_metadata::LaunchMetadata;

pub const APP_LOG_DIR: &str = "APP_LOG_DIR";
pub const DISABLE_WASMTIME_CACHE: &str = "DISABLE_WASMTIME_CACHE";
pub const FOLLOW_LOG_OPT: &str = "FOLLOW_ID";
pub const WASMTIME_CACHE_FILE: &str = "WASMTIME_CACHE_FILE";
pub const RUNTIME_CONFIG_FILE: &str = "RUNTIME_CONFIG_FILE";

// Set by `spin up`
pub const SPIN_LOCKED_URL: &str = "SPIN_LOCKED_URL";
pub const SPIN_LOCAL_APP_DIR: &str = "SPIN_LOCAL_APP_DIR";
pub const SPIN_WORKING_DIR: &str = "SPIN_WORKING_DIR";

/// A command that runs a TriggerExecutor.
#[derive(Parser, Debug)]
#[clap(
    usage = "spin [COMMAND] [OPTIONS]",
    next_help_heading = help_heading::<Executor>()
)]
pub struct TriggerExecutorCommand<Executor: TriggerExecutor>
where
    Executor::RunConfig: Args,
{
    /// Log directory for the stdout and stderr of components. Setting to
    /// the empty string disables logging to disk.
    #[clap(
        name = APP_LOG_DIR,
        short = 'L',
        long = "log-dir",
        env = "SPIN_LOG_DIR",
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

    /// Disable Wasmtime's pooling instance allocator.
    #[clap(long = "disable-pooling")]
    pub disable_pooling: bool,

    /// Print output to stdout/stderr only for given component(s)
    #[clap(
        name = FOLLOW_LOG_OPT,
        long = "follow",
        multiple_occurrences = true,
    )]
    pub follow_components: Vec<String>,

    /// Silence all component output to stdout/stderr
    #[clap(
        long = "quiet",
        short = 'q',
        aliases = &["sh", "shush"],
        conflicts_with = FOLLOW_LOG_OPT,
        )]
    pub silence_component_logs: bool,

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

    /// Set the application state directory path. This is used in the default
    /// locations for logs, key value stores, etc.
    ///
    /// For local apps, this defaults to `.spin/` relative to the `spin.toml` file.
    /// For remote apps, this has no default (unset).
    /// Passing an empty value forces the value to be unset.
    #[clap(long)]
    pub state_dir: Option<String>,

    #[clap(flatten)]
    pub run_config: Executor::RunConfig,

    /// Set a key/value pair (key=value) in the application's
    /// default store. Any existing value will be overwritten.
    /// Can be used multiple times.
    #[clap(long = "key-value", parse(try_from_str = parse_kv))]
    key_values: Vec<(String, String)>,

    /// Run a SQLite statement such as a migration against the default database.
    /// To run from a file, prefix the filename with @ e.g. spin up --sqlite @migration.sql
    #[clap(long = "sqlite")]
    sqlite_statements: Vec<String>,

    #[clap(long = "help-args-only", hide = true)]
    pub help_args_only: bool,

    #[clap(long = "launch-metadata-only", hide = true)]
    pub launch_metadata_only: bool,
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

        if self.launch_metadata_only {
            let lm = LaunchMetadata::infer::<Executor>();
            let json = serde_json::to_string_pretty(&lm)?;
            eprintln!("{json}");
            return Ok(());
        }

        // Required env vars
        let working_dir = std::env::var(SPIN_WORKING_DIR).context(SPIN_WORKING_DIR)?;
        let locked_url = std::env::var(SPIN_LOCKED_URL).context(SPIN_LOCKED_URL)?;

        let init_data = crate::HostComponentInitData::new(
            &*self.key_values,
            &*self.sqlite_statements,
            LLmOptions { use_gpu: true },
        );

        let loader = TriggerLoader::new(working_dir, self.allow_transient_write);
        let executor = self.build_executor(loader, locked_url, init_data).await?;

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

    async fn build_executor(
        &self,
        loader: impl Loader + Send + Sync + 'static,
        locked_url: String,
        init_data: crate::HostComponentInitData,
    ) -> Result<Executor> {
        let runtime_config = self.build_runtime_config()?;

        let _sloth_guard = warn_if_wasm_build_slothful();

        let mut builder = TriggerExecutorBuilder::new(loader);
        self.update_config(builder.config_mut())?;

        builder.hooks(StdioLoggingTriggerHooks::new(self.follow_components()));
        builder.hooks(Network::default());
        builder.hooks(SummariseRuntimeConfigHook::new(&self.runtime_config_file));
        builder.hooks(KeyValuePersistenceMessageHook);
        builder.hooks(SqlitePersistenceMessageHook);

        builder.build(locked_url, runtime_config, init_data).await
    }

    fn build_runtime_config(&self) -> Result<RuntimeConfig> {
        let local_app_dir = std::env::var_os(SPIN_LOCAL_APP_DIR);
        let mut config = RuntimeConfig::new(local_app_dir.map(Into::into));
        if let Some(state_dir) = &self.state_dir {
            config.set_state_dir(state_dir);
        }
        if let Some(log_dir) = &self.log {
            config.set_log_dir(log_dir);
        }
        if let Some(config_file) = &self.runtime_config_file {
            config.merge_config_file(config_file)?;
        }
        Ok(config)
    }

    fn follow_components(&self) -> FollowComponents {
        if self.silence_component_logs {
            FollowComponents::None
        } else if self.follow_components.is_empty() {
            FollowComponents::All
        } else {
            let followed = self.follow_components.clone().into_iter().collect();
            FollowComponents::Named(followed)
        }
    }

    fn update_config(&self, config: &mut spin_core::Config) -> Result<()> {
        // Apply --cache / --disable-cache
        if !self.disable_cache {
            config.enable_cache(&self.cache)?;
        }

        if self.disable_pooling {
            config.disable_pooling();
        }

        Ok(())
    }
}

const SLOTH_WARNING_DELAY_MILLIS: u64 = 1250;

fn warn_if_wasm_build_slothful() -> sloth::SlothGuard {
    #[cfg(debug_assertions)]
    let message = "\
        This is a debug build - preparing Wasm modules might take a few seconds\n\
        If you're experiencing long startup times please switch to the release build";

    #[cfg(not(debug_assertions))]
    let message = "Preparing Wasm modules is taking a few seconds...";

    sloth::warn_if_slothful(SLOTH_WARNING_DELAY_MILLIS, format!("{message}\n"))
}

fn help_heading<E: TriggerExecutor>() -> Option<&'static str> {
    if E::TRIGGER_TYPE == help::HelpArgsOnlyTrigger::TRIGGER_TYPE {
        Some("TRIGGER OPTIONS")
    } else {
        let heading = format!("{} TRIGGER OPTIONS", E::TRIGGER_TYPE.to_uppercase());
        let as_str = Box::new(heading).leak();
        Some(as_str)
    }
}

pub mod help {
    use super::*;

    /// Null object to support --help-args-only in the absence of
    /// a `spin.toml` file.
    pub struct HelpArgsOnlyTrigger;

    #[async_trait::async_trait]
    impl TriggerExecutor for HelpArgsOnlyTrigger {
        const TRIGGER_TYPE: &'static str = "help-args-only";
        type RuntimeData = ();
        type TriggerConfig = ();
        type RunConfig = NoArgs;
        type InstancePre = spin_core::InstancePre<Self::RuntimeData>;
        async fn new(_: crate::TriggerAppEngine<Self>) -> Result<Self> {
            Ok(Self)
        }
        async fn run(self, _: Self::RunConfig) -> Result<()> {
            Ok(())
        }
    }
}
