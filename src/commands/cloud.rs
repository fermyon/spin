use anyhow::Result;
use clap::Subcommand;

use super::deploy::DeployCommand;
use super::login::LoginCommand;

/// Commands for publishing applications to the Fermyon Platform.
#[derive(Subcommand, Debug)]
pub enum CloudCommands {
    /// Package and upload an application to the Fermyon Platform.
    Deploy(DeployCommand),

    /// Log into the Fermyon Platform.
    Login(LoginCommand),
}

impl CloudCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            Self::Deploy(cmd) => cmd.run().await,
            Self::Login(cmd) => cmd.run().await,
        }
    }
}
