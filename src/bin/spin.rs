use anyhow::Error;
use spin_cli::commands::{
    bindle::BindleCommands, new::NewCommand, templates::TemplateCommands, up::Up,
};
use structopt::{clap::AppSettings, StructOpt};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    SpinApp::from_args().run().await
}

/// The Spin CLI
#[derive(Debug, StructOpt)]
#[structopt(
    name = "spin",
    version = env!("CARGO_PKG_VERSION"),
    global_settings = &[
        AppSettings::VersionlessSubcommands,
        AppSettings::ColoredHelp
    ])]
enum SpinApp {
    Templates(TemplateCommands),
    New(NewCommand),
    Up(Up),
    Bindle(BindleCommands),
}

impl SpinApp {
    /// The main entry point to Spin.
    pub async fn run(self) -> Result<(), Error> {
        match self {
            SpinApp::Templates(cmd) => cmd.run().await,
            SpinApp::Up(cmd) => cmd.run().await,
            SpinApp::New(cmd) => cmd.run().await,
            SpinApp::Bindle(cmd) => cmd.run().await,
        }
    }
}
