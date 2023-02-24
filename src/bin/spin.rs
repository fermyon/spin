use anyhow::Error;
use clap::{CommandFactory, Parser, Subcommand};
use is_terminal::IsTerminal;
use lazy_static::lazy_static;
use spin_cli::commands::{
    bindle::BindleCommands,
    build::BuildCommand,
    cloud::CloudCommands,
    deploy::DeployCommand,
    external::execute_external_subcommand,
    login::LoginCommand,
    new::{AddCommand, NewCommand},
    plugins::PluginCommands,
    registry::RegistryCommands,
    templates::TemplateCommands,
    up::UpCommand,
};
use spin_http::HttpTrigger;
use spin_redis_engine::RedisTrigger;
use spin_trigger::cli::help::HelpArgsOnlyTrigger;
use spin_trigger::cli::TriggerExecutorCommand;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(std::io::stderr().is_terminal())
        .init();
    SpinApp::parse().run().await
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
#[command(
    name = "spin",
    version = version(),
    next_display_order = None
)]
enum SpinApp {
    #[command(subcommand, alias = "template")]
    Templates(TemplateCommands),
    New(NewCommand),
    Add(AddCommand),
    Up(UpCommand),
    #[command(subcommand)]
    Bindle(BindleCommands),
    #[command(subcommand)]
    Cloud(CloudCommands),
    // acts as a cross-level subcommand shortcut -> `spin cloud deploy`
    Deploy(DeployCommand),
    // acts as a cross-level subcommand shortcut -> `spin cloud login`
    Login(LoginCommand),
    #[command(subcommand, alias = "oci")]
    Registry(RegistryCommands),
    Build(BuildCommand),
    #[command(subcommand, alias = "plugin")]
    Plugins(PluginCommands),
    #[command(subcommand, hide = true)]
    Trigger(TriggerCommands),
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Subcommand)]
#[command(next_display_order = None, ignore_errors=true)]
enum TriggerCommands {
    Http(TriggerExecutorCommand<HttpTrigger>),
    Redis(TriggerExecutorCommand<RedisTrigger>),
    #[command(name = spin_cli::HELP_ARGS_ONLY_TRIGGER_TYPE, hide = true)]
    HelpArgsOnly(TriggerExecutorCommand<HelpArgsOnlyTrigger>),
}

impl SpinApp {
    /// The main entry point to Spin.
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Templates(cmd) => cmd.run().await,
            Self::Up(cmd) => cmd.run().await,
            Self::New(cmd) => cmd.run().await,
            Self::Add(cmd) => cmd.run().await,
            Self::Bindle(cmd) => cmd.run().await,
            Self::Cloud(cmd) => cmd.run().await,
            Self::Deploy(cmd) => cmd.run().await,
            Self::Login(cmd) => cmd.run().await,
            Self::Registry(cmd) => cmd.run().await,
            Self::Build(cmd) => cmd.run().await,
            Self::Trigger(TriggerCommands::Http(cmd)) => cmd.run().await,
            Self::Trigger(TriggerCommands::Redis(cmd)) => cmd.run().await,
            Self::Trigger(TriggerCommands::HelpArgsOnly(cmd)) => cmd.run().await,
            Self::Plugins(cmd) => cmd.run().await,
            Self::External(cmd) => execute_external_subcommand(cmd, SpinApp::command()).await,
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
