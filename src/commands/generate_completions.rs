use anyhow::Error;
use async_trait::async_trait;
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use clap_complete_fig::Fig;

use crate::dispatch::Dispatch;

#[derive(Subcommand)]
pub enum GenerateCompletionsCommands {
    Shell(GenerateCompletionsShellCommand),
    Fig(GenerateCompletionsFigCommand),
}

#[async_trait(?Send)]
impl Dispatch for GenerateCompletionsCommands {
    async fn run(&self) -> Result<(), Error> {
        match self {
            Self::Shell(cmd) => cmd.run().await,
            Self::Fig(cmd) => cmd.run().await,
        }
    }
}

#[derive(Args, Clone, Debug)]
pub struct GenerateCompletionsArgs {
    /// Shell to generate completions for
    shell: Shell,
    /// Generate completions for fig
    fig: bool,
}

/// Generate Fig completions
#[derive(Parser, Debug)]
pub struct GenerateCompletionsFigCommand;

#[async_trait(?Send)]
impl Dispatch for GenerateCompletionsFigCommand {
    async fn run(&self) -> Result<(), Error> {
        Self::print_completions(Fig);
        Ok(())
    }
}

/// Generate Shell completions
#[derive(Parser, Debug)]
pub struct GenerateCompletionsShellCommand {
    #[arg(value_parser = clap::value_parser!(clap_complete::Shell))]
    pub shell: clap_complete::Shell,
}

#[async_trait(?Send)]
impl Dispatch for GenerateCompletionsShellCommand {
    async fn run(&self) -> Result<(), Error> {
        Self::print_completions(self.shell);
        Ok(())
    }
}

trait Completions: CommandFactory {
    fn print_completions<G: Generator>(gen: G) {
        generate(gen, &mut Self::command(), "spin", &mut std::io::stdout())
    }
}

impl<T: CommandFactory> Completions for T {}
