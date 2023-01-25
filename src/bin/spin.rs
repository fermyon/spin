use anyhow::Result;
use async_trait::async_trait;
use clap::{Parser, Subcommand, CommandFactory};
use is_terminal::IsTerminal;
use lazy_static::lazy_static;
use spin_cli::{commands::{
    bindle::BindleCommands,
    build::BuildCommand,
    deploy::DeployCommand,
    login::LoginCommand,
    new::{AddCommand, NewCommand},
    oci::OciCommands,
    plugins::PluginCommands,
    templates::TemplateCommands,
    up::UpCommand, external::ExternalCommands,
}, dispatch::Action};
use spin_cli::dispatch::{Dispatch};
use anyhow::anyhow;
use spin_cli::*;
use spin_http::HttpTrigger;
use spin_redis_engine::RedisTrigger;
use spin_trigger::cli::help::HelpArgsOnlyTrigger;
use spin_trigger::cli::TriggerExecutorCommand;

#[cfg(feature = "generate-completions")]
use spin_cli::commands::generate_completions::GenerateCompletionsCommands;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(std::io::stderr().is_terminal())
        .init();
    SpinApp::parse().dispatch(&Action::Help).await?;

    Ok(())
}

lazy_static! {
    pub static ref VERSION: String = build_info();
}

/// Helper for passing VERSION to structopt.
fn version() -> &'static str {
    &VERSION
}

/// The Spin CLI
#[derive(Parser)]
#[command(id = "spin", version = version())]
enum SpinApp {
    #[command(subcommand, alias = "template")]
    Templates(TemplateCommands),
    New(NewCommand),
    Add(AddCommand),
    Up(UpCommand),
    #[command(subcommand)]
    Bindle(BindleCommands),
    #[command(subcommand)]
    Oci(OciCommands),
    Deploy(DeployCommand),
    Build(BuildCommand),
    Login(LoginCommand),
    #[command(subcommand, alias = "plugin")]
    Plugins(PluginCommands),
    #[cfg(feature = "generate-completions")]
    /// Generate shell completions
    #[command(subcommand, hide = true)]
    GenerateCompletions(GenerateCompletionsCommands),
    #[command(subcommand, hide = true)]
    Trigger(TriggerCommands),
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Subcommand)]
enum TriggerCommands {
    Http(TriggerExecutorCommand<HttpTrigger>),
    Redis(TriggerExecutorCommand<RedisTrigger>),
    #[clap(name = spin_cli::HELP_ARGS_ONLY_TRIGGER_TYPE, hide = true)]
    HelpArgsOnly(TriggerExecutorCommand<HelpArgsOnlyTrigger>),
}

impl_dispatch!(TriggerCommands::{Http, Redis, HelpArgsOnly});

#[async_trait(?Send)]
impl Dispatch for SpinApp {
    /// The main entry point to Spin.
    async fn dispatch(&self, action: &Action) -> Result<()> {
        match self {
            Self::Templates(cmd) => cmd.dispatch(action).await,
            Self::Up(cmd) => cmd.dispatch(action).await,
            Self::New(cmd) => cmd.dispatch(action).await,
            Self::Add(cmd) => cmd.dispatch(action).await,
            Self::Bindle(cmd) => cmd.dispatch(action).await,
            Self::Oci(cmd) => cmd.dispatch(action).await,
            Self::Deploy(cmd) => cmd.dispatch(action).await,
            Self::Build(cmd) => cmd.dispatch(action).await,
            Self::Trigger(cmd) => match_action!(cmd[action].await),
            Self::Login(cmd) => cmd.dispatch(action).await,
            Self::Plugins(cmd) => cmd.dispatch(action).await,
            #[cfg(feature = "generate-completions")]
            Self::GenerateCompletions(cmd) => cmd.dispatch(action).await,
            Self::External(cmd) => {
                ExternalCommands::new(cmd.to_vec(), SpinApp::command())
                    .dispatch(action).await
            } // execute_external_subcommand(cmd, SpinApp::command()),
        }
    }
}

/// Returns build information, similar to: 0.1.0 (2be4034 2022-03-31).
fn build_info() -> String {
    format!(
        "{} ({} {})",
        env!("VERGEN_BUILD_SEMVER"),
        env!("VERGEN_GIT_SHA_SHORT"),
        env!("VERGEN_GIT_COMMIT_DATE")
    )
}
