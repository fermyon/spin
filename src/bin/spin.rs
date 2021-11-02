use anyhow::Error;
use spin_cli::commands::templates::TemplatesCommand;
use structopt::{clap::AppSettings, StructOpt};

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    SpinApp::from_args().run().await
}

/// The Spin CLI
#[derive(StructOpt)]
#[structopt(
    name = "spin",
    version = env!("CARGO_PKG_VERSION"),
    global_settings = &[
        AppSettings::VersionlessSubcommands,
        AppSettings::ColoredHelp
    ])]
enum SpinApp {
    Templates(TemplatesCommand),
}

impl SpinApp {
    /// The main entry point to Spin.
    pub async fn run(self) -> Result<(), Error> {
        match self {
            SpinApp::Templates(t) => t.run().await,
        }
    }
}
