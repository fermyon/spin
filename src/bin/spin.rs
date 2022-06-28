use anyhow::Error;
use clap::{Parser, Subcommand};
use lazy_static::lazy_static;
use spin_cli::commands::{
    bindle::BindleCommands, build::BuildCommand, deploy::DeployCommand, new::NewCommand,
    templates::TemplateCommands, up::UpCommand,
};
use spin_http_engine::HttpTrigger;
use spin_redis_engine::RedisTrigger;
use spin_trigger::cli::TriggerExecutorCommand;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};

#[tokio::main]
async fn main() -> Result<(), Error> {
    Cli::parse().run().await
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
struct Cli {
    /// Turn debugging information on
    #[clap(short, parse(from_occurrences))]
    verbose: u8,

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn run(self) -> Result<(), Error> {
        let filter = EnvFilter::builder()
            .with_env_var("RUST_LOG")
            .with_default_directive(match self.verbose {
                0 => LevelFilter::OFF.into(),
                1 => LevelFilter::ERROR.into(),
                2 => LevelFilter::WARN.into(),
                3 => LevelFilter::INFO.into(),
                4 => LevelFilter::DEBUG.into(),
                _ => LevelFilter::TRACE.into(),
            })
            .parse("")?;

        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(filter)
            .with_ansi(atty::is(atty::Stream::Stderr))
            .init();

        self.command.run().await
    }
}

#[derive(Subcommand)]
enum Commands {
    #[clap(subcommand)]
    Templates(TemplateCommands),
    New(NewCommand),
    Up(UpCommand),
    #[clap(subcommand)]
    Bindle(BindleCommands),
    Deploy(DeployCommand),
    Build(BuildCommand),
    #[clap(subcommand, hide = true)]
    Trigger(TriggerCommands),
}

#[derive(Subcommand)]
enum TriggerCommands {
    Http(TriggerExecutorCommand<HttpTrigger>),
    Redis(TriggerExecutorCommand<RedisTrigger>),
}

impl Commands {
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
