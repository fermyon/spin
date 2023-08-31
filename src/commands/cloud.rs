use crate::commands::external::execute_external_subcommand;
use anyhow::Result;
use clap::Args;

#[derive(Debug, Args, PartialEq)]
#[clap(
    about = "Package and upload an application to the Fermyon Cloud.",
    allow_hyphen_values = true,
    disable_help_flag = true
)]
pub struct DeployCommand {
    /// All args to be passed through to the plugin
    #[clap(hide = true)]
    args: Vec<String>,
}

#[derive(Debug, Args, PartialEq)]
#[clap(
    about = "Log into the Fermyon Cloud.",
    allow_hyphen_values = true,
    disable_help_flag = true
)]
pub struct LoginCommand {
    /// All args to be passed through to the plugin
    #[clap(hide = true)]
    args: Vec<String>,
}

impl DeployCommand {
    pub async fn run(self, app: clap::App<'_>) -> Result<()> {
        let mut cmd = vec!["cloud".to_string(), "deploy".to_string()];
        cmd.append(&mut self.args.clone());
        execute_external_subcommand(cmd, app).await
    }
}

impl LoginCommand {
    pub async fn run(self, app: clap::App<'_>) -> Result<()> {
        let mut cmd = vec!["cloud".to_string(), "login".to_string()];
        cmd.append(&mut self.args.clone());
        execute_external_subcommand(cmd, app).await
    }
}
