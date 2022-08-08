use anyhow::Result;
use tokio::process::Command;
use tracing::log;

use crate::commands::plugins::get_spin_plugins_directory;

// TODO: Add capability to distinguish between standalone binaries and pluigns
// TODO: Should this be a struct to maintain consistency across subcommands?

pub async fn execute_external_subcommand(args: Vec<String>) -> Result<()> {
    // TODO: What environmental variables should be passed.

    let path = get_spin_plugins_directory()?
        .join(args.first().unwrap())
        .join(args.first().unwrap());
    let mut command = Command::new(path);
    if args.len() > 1 {
        command.args(&args[1..]);
    }
    log::info!("Executing command {:?}", command);
    // Allow user to interact with stdio/stdout of child process
    let _ = command.status().await?;
    // TODO: handle the status

    Ok(())
}
