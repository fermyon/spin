use crate::build_info::*;
use crate::commands::{
    build::BuildCommand,
    cloud::{DeployCommand, LoginCommand},
    doctor::DoctorCommand,
    external::execute_external_subcommand,
    new::{AddCommand, NewCommand},
    plugins::PluginCommands,
    registry::RegistryCommands,
    templates::TemplateCommands,
    up::UpCommand,
    watch::WatchCommand,
};
use anyhow::Error;
use clap::{CommandFactory, Parser, Subcommand};
use lazy_static::lazy_static;
use spin_redis_engine::RedisTrigger;
use spin_trigger::cli::help::HelpArgsOnlyTrigger;
use spin_trigger::cli::TriggerExecutorCommand;
use spin_trigger_http::HttpTrigger;

pub mod build_info;
pub mod commands;
pub mod opts;
pub mod subprocess;

pub use opts::HELP_ARGS_ONLY_TRIGGER_TYPE;

/// The Spin CLI
#[derive(Parser)]
#[clap(
    name = "spin",
    version = version()
)]
pub enum SpinApp {
    #[clap(subcommand, alias = "template")]
    Templates(TemplateCommands),
    New(NewCommand),
    Add(AddCommand),
    Up(UpCommand),
    // acts as a cross-level subcommand shortcut -> `spin cloud deploy`
    Deploy(DeployCommand),
    // acts as a cross-level subcommand shortcut -> `spin cloud login`
    Login(LoginCommand),
    #[clap(subcommand, alias = "oci")]
    Registry(RegistryCommands),
    Build(BuildCommand),
    #[clap(subcommand, alias = "plugin")]
    Plugins(PluginCommands),
    #[clap(subcommand, hide = true)]
    Trigger(TriggerCommands),
    #[clap(external_subcommand)]
    External(Vec<String>),
    Watch(WatchCommand),
    Doctor(DoctorCommand),
}

#[derive(Subcommand)]
pub enum TriggerCommands {
    Http(TriggerExecutorCommand<HttpTrigger>),
    Redis(TriggerExecutorCommand<RedisTrigger>),
    #[clap(name = HELP_ARGS_ONLY_TRIGGER_TYPE, hide = true)]
    HelpArgsOnly(TriggerExecutorCommand<HelpArgsOnlyTrigger>),
}

impl SpinApp {
    /// The main entry point to Spin.
    pub async fn run(self, app: clap::Command<'_>) -> Result<(), Error> {
        match self {
            Self::Templates(cmd) => cmd.run().await,
            Self::Up(cmd) => cmd.run().await,
            Self::New(cmd) => cmd.run().await,
            Self::Add(cmd) => cmd.run().await,
            Self::Deploy(cmd) => cmd.run(SpinApp::command()).await,
            Self::Login(cmd) => cmd.run(SpinApp::command()).await,
            Self::Registry(cmd) => cmd.run().await,
            Self::Build(cmd) => cmd.run().await,
            Self::Trigger(TriggerCommands::Http(cmd)) => cmd.run().await,
            Self::Trigger(TriggerCommands::Redis(cmd)) => cmd.run().await,
            Self::Trigger(TriggerCommands::HelpArgsOnly(cmd)) => cmd.run().await,
            Self::Plugins(cmd) => cmd.run().await,
            Self::External(cmd) => execute_external_subcommand(cmd, app).await,
            Self::Watch(cmd) => cmd.run().await,
            Self::Doctor(cmd) => cmd.run().await,
        }
    }
}

lazy_static! {
    pub static ref VERSION: String = build_info();
}

/// Helper for passing VERSION to structopt.
fn version() -> &'static str {
    &VERSION
}

/// Returns build information, similar to: 0.1.0 (2be4034 2022-03-31).
fn build_info() -> String {
    format!("{SPIN_VERSION} ({SPIN_COMMIT_SHA} {SPIN_COMMIT_DATE})")
}
