use anyhow::Error;
use lazy_static::lazy_static;
use spin_cli::commands::{
    bindle::BindleCommands, deploy::DeployCommand, new::NewCommand, templates::TemplateCommands,
    up::UpCommand,
};
use structopt::{clap::AppSettings, StructOpt};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    SpinApp::from_args().run().await
}

lazy_static! {
    pub static ref VERSION: String = build_info();
}

/// Helper for passing VERSION to structopt.
fn version() -> &'static str {
    &VERSION
}

/// The Spin CLI
#[derive(Debug, StructOpt)]
#[structopt(
    name = "spin",
    version = version(),
    global_settings = &[
        AppSettings::VersionlessSubcommands,
        AppSettings::ColoredHelp
    ])]
enum SpinApp {
    Templates(TemplateCommands),
    New(NewCommand),
    Up(UpCommand),
    Bindle(BindleCommands),
    Deploy(DeployCommand),
}

impl SpinApp {
    /// The main entry point to Spin.
    pub async fn run(self) -> Result<(), Error> {
        match self {
            SpinApp::Templates(cmd) => cmd.run().await,
            SpinApp::Up(cmd) => cmd.run().await,
            SpinApp::New(cmd) => cmd.run().await,
            SpinApp::Bindle(cmd) => cmd.run().await,
            SpinApp::Deploy(cmd) => cmd.run().await,
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
