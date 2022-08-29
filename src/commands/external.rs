use anyhow::{anyhow, Result};
use spin_plugins::version_check::check_plugin_spin_compatibility;
use tokio::process::Command;
use tracing::log;

use crate::commands::plugins::get_spin_plugins_directory;

// TODO: Add capability to distinguish between standalone binaries and plugins
// TODO: Should this be a struct to maintain consistency across subcommands?

pub async fn execute_external_subcommand(args: Vec<String>) -> Result<()> {
    // TODO: What environmental variables should be passed.
    let plugin_name = args.first().ok_or_else(|| anyhow!("Expected subcommand"))?;
    let plugins_dir = get_spin_plugins_directory()?;
    check_plugin_spin_compatibility(plugin_name, env!("VERGEN_BUILD_SEMVER"), &plugins_dir)?;
    let path = plugins_dir.join(plugin_name).join(plugin_name);
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
