use anyhow::Result;
use clap::Parser;

/// Generate shell completions.
#[derive(Parser, Debug)]
#[clap(about = "Generate completions")]
pub struct GenerateCompletionsCommand {
    #[clap(value_parser = clap::value_parser!(clap_complete::Shell))]
    pub shell: clap_complete::Shell,
}

impl GenerateCompletionsCommand {
    pub async fn run(&self, mut cmd: clap::Command<'_>) -> Result<()> {
        // let mut cmd: clap::Command = SpinApp::into_app();
        print_completions(self.shell, &mut cmd);
        Ok(())
    }
}

fn print_completions<G: clap_complete::Generator>(gen: G, cmd: &mut clap::Command) {
    clap_complete::generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout())
}
