mod launch_metadata;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, IntoApp, Parser};
use spin_app::App;
use spin_common::ui::quoted_path;
use spin_common::url::parse_file_url;
use spin_common::{arg_parser::parse_kv, sloth};
use spin_factors_executor::{ComponentLoader, FactorsExecutor};
use spin_runtime_config::ResolvedRuntimeConfig;

use crate::factors::{TriggerFactors, TriggerFactorsRuntimeConfig};
use crate::stdio::{FollowComponents, StdioLoggingExecutorHooks};
use crate::Trigger;
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
    next_help_heading = help_heading::<T>()
)]
pub struct FactorsTriggerCommand<T: Trigger> {
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
    pub trigger_args: T::CliArgs,

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

impl<T: Trigger> FactorsTriggerCommand<T> {
    /// Create a new TriggerExecutorBuilder from this TriggerExecutorCommand.
    pub async fn run(self) -> Result<()> {
        // Handle --help-args-only
        if self.help_args_only {
            Self::command()
                .disable_help_flag(true)
                .help_template("{all-args}")
                .print_long_help()?;
            return Ok(());
        }

        // Handle --launch-metadata-only
        if self.launch_metadata_only {
            let lm = LaunchMetadata::infer::<T>();
            let json = serde_json::to_string_pretty(&lm)?;
            eprintln!("{json}");
            return Ok(());
        }

        // Required env vars
        let working_dir = std::env::var(SPIN_WORKING_DIR).context(SPIN_WORKING_DIR)?;
        let locked_url = std::env::var(SPIN_LOCKED_URL).context(SPIN_LOCKED_URL)?;

        let follow_components = self.follow_components();

        // Load App
        let app = {
            let path = parse_file_url(&locked_url)?;
            let contents = std::fs::read(&path)
                .with_context(|| format!("failed to read manifest at {}", quoted_path(&path)))?;
            let locked =
                serde_json::from_slice(&contents).context("failed to parse app lock file JSON")?;
            App::new(locked_url, locked)
        };

        // Validate required host features
        if let Err(unmet) = app.ensure_needs_only(&T::supported_host_requirements()) {
            anyhow::bail!("This application requires the following features that are not available in this version of the '{}' trigger: {unmet}", T::TYPE);
        }

        let mut trigger = T::new(self.trigger_args, &app)?;

        let mut core_engine_builder = {
            let mut config = spin_core::Config::default();

            // Apply --cache / --disable-cache
            if !self.disable_cache {
                config.enable_cache(&self.cache)?;
            }

            if self.disable_pooling {
                config.disable_pooling();
            }

            trigger.update_core_config(&mut config)?;

            spin_core::Engine::builder(&config)?
        };
        trigger.add_to_linker(core_engine_builder.linker())?;

        let runtime_config = match &self.runtime_config_file {
            Some(runtime_config_path) => {
                ResolvedRuntimeConfig::<TriggerFactorsRuntimeConfig>::from_file(
                    runtime_config_path,
                    self.state_dir.as_deref(),
                )?
            }
            None => ResolvedRuntimeConfig::default(),
        };

        runtime_config
            .set_initial_key_values(&self.key_values)
            .await?;

        let factors = TriggerFactors::new(
            working_dir,
            self.allow_transient_write,
            runtime_config.key_value_resolver,
        );

        // TODO: move these into Factor methods/constructors
        // let init_data = crate::HostComponentInitData::new(
        //     &*self.key_values,
        //     &*self.sqlite_statements,
        //     LLmOptions { use_gpu: true },
        // );

        // TODO: component loader
        struct TodoComponentLoader;
        impl ComponentLoader for TodoComponentLoader {
            fn load_component(
                &mut self,
                _engine: &spin_core::wasmtime::Engine,
                _component: &spin_factors::AppComponent,
            ) -> anyhow::Result<spin_core::Component> {
                todo!()
            }
        }

        let mut executor = FactorsExecutor::new(core_engine_builder, factors)?;

        let log_dir = self.log.clone();
        executor.add_hooks(StdioLoggingExecutorHooks::new(follow_components, log_dir));
        // TODO:
        // builder.hooks(SummariseRuntimeConfigHook::new(&self.runtime_config_file));
        // builder.hooks(KeyValuePersistenceMessageHook);
        // builder.hooks(SqlitePersistenceMessageHook);

        let configured_app = {
            let _sloth_guard = warn_if_wasm_build_slothful();
            executor.load_app(app, runtime_config.runtime_config, TodoComponentLoader)?
        };

        let run_fut = trigger.run(configured_app);

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

fn help_heading<T: Trigger>() -> Option<&'static str> {
    if T::TYPE == help::HelpArgsOnlyTrigger::TYPE {
        Some("TRIGGER OPTIONS")
    } else {
        let heading = format!("{} TRIGGER OPTIONS", T::TYPE.to_uppercase());
        let as_str = Box::new(heading).leak();
        Some(as_str)
    }
}

pub mod help {
    use super::*;

    /// Null object to support --help-args-only in the absence of
    /// a `spin.toml` file.
    pub struct HelpArgsOnlyTrigger;

    impl Trigger for HelpArgsOnlyTrigger {
        const TYPE: &'static str = "help-args-only";
        type CliArgs = NoArgs;
        type InstanceState = ();

        fn new(_cli_args: Self::CliArgs, _app: &App) -> anyhow::Result<Self> {
            Ok(Self)
        }

        async fn run(
            self,
            _configured_app: spin_factors_executor::FactorsExecutorApp<
                TriggerFactors,
                Self::InstanceState,
            >,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }
}
