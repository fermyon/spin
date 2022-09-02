use crate::commands::plugins::get_spin_plugins_directory;

use anyhow::{anyhow, Result};
use spin_plugins::version_check::check_plugin_spin_compatibility;
use std::{collections::HashMap, env, path::Path};
use tokio::process::Command;
use tracing::log;

/// Executes a Spin plugin as a subprocess, expecting the first argument to
/// indicate the plugin to execute. Passes all subsequent arguments on to the
/// subprocess.
pub async fn execute_external_subcommand(args: Vec<String>) -> Result<()> {
    let plugin_name = args.first().ok_or_else(|| anyhow!("Expected subcommand"))?;
    let plugins_dir = get_spin_plugins_directory()?;
    check_plugin_spin_compatibility(plugin_name, env!("VERGEN_BUILD_SEMVER"), &plugins_dir)?;
    let path = plugins_dir.join(plugin_name);
    let mut binary = path.join(plugin_name);
    if cfg!(target_os = "windows") {
        binary.set_extension("exe");
    }
    let mut command = Command::new(binary);
    if args.len() > 1 {
        command.args(&args[1..]);
    }
    command.envs(&get_env_vars_map(&path)?);
    log::info!("Executing command {:?}", command);
    // Allow user to interact with stdio/stdout of child process
    let status = command.status().await?;
    log::info!("Exiting process with {}", status);
    Ok(())
}

fn get_env_vars_map(path: &Path) -> Result<HashMap<String, String>> {
    let map: HashMap<String, String> = vec![
        (
            "SPIN_VERSION".to_string(),
            env!("VERGEN_BUILD_SEMVER").to_owned(),
        ),
        (
            "SPIN_BIN_PATH".to_string(),
            env::current_exe()?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert binary path to string"))?
                .to_string(),
        ),
        (
            "SPIN_PLUGIN_PATH".to_string(),
            path.to_str()
                .ok_or_else(|| anyhow!("Could not convert plugin path to string"))?
                .to_string(),
        ),
    ]
    .into_iter()
    .collect();
    Ok(map)
}
