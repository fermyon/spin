use anyhow::Error;
use clap::Parser;
use lazy_static::lazy_static;

use spin_cli::commands::{
    bindle::BindleCommands, build::BuildCommand, deploy::DeployCommand, new::NewCommand,
    templates::TemplateCommands, up::UpCommand,
};

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
#[derive(Parser, Debug)]
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
