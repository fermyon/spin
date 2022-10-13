use anyhow::Error;
use clap::{CommandFactory, Parser, Subcommand};
use lazy_static::lazy_static;
use spin_cli::commands::{
    bindle::BindleCommands, build::BuildCommand, deploy::DeployCommand,
    external::execute_external_subcommand, login::LoginCommand, new::NewCommand,
    plugins::PluginCommands, templates::TemplateCommands, up::UpCommand,
};
use spin_http::HttpTrigger;
use spin_redis_engine::RedisTrigger;
use spin_trigger::cli::TriggerExecutorCommand;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(atty::is(atty::Stream::Stderr))
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
#[clap(
    name = "spin",
    version = version(),
)]
enum SpinApp {
    #[clap(subcommand)]
    Templates(TemplateCommands),
    New(NewCommand),
    Up(UpCommand),
    #[clap(subcommand)]
    Bindle(BindleCommands),
    Deploy(DeployCommand),
    Build(BuildCommand),
    Login(LoginCommand),
    #[clap(subcommand)]
    Plugin(PluginCommands),
    #[clap(subcommand, hide = true)]
    Trigger(TriggerCommands),
    #[clap(external_subcommand)]
    External(Vec<String>),
}

#[derive(Subcommand)]
enum TriggerCommands {
    Http(TriggerExecutorCommand<HttpTrigger>),
    Redis(TriggerExecutorCommand<RedisTrigger>),
}

impl SpinApp {
    /// The main entry point to Spin.
    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::Templates(cmd) => cmd.run().await,
            Self::Up(cmd) => cmd.run().await,
            Self::New(cmd) => cmd.run().await,
            Self::Bindle(cmd) => cmd.run().await,
            Self::Deploy(cmd) => cmd.run().await,
            Self::Build(cmd) => cmd.run().await,
            Self::Trigger(TriggerCommands::Http(cmd)) => cmd.run().await,
            Self::Trigger(TriggerCommands::Redis(cmd)) => cmd.run().await,
            Self::Login(cmd) => cmd.run().await,
            Self::Plugin(cmd) => cmd.run().await,
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
