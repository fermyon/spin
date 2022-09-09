use anyhow::{anyhow, Result};
use clap::App;
use spin_plugins::{manifest::check_supported_version, PluginStore};
use std::{collections::HashMap, env, process};
use tokio::process::Command;
use tracing::log;

/// Executes a Spin plugin as a subprocess, expecting the first argument to
/// indicate the plugin to execute. Passes all subsequent arguments on to the
/// subprocess.
pub async fn execute_external_subcommand(args: Vec<String>, app: App<'_>) -> Result<()> {
    let plugin_name = args.first().ok_or_else(|| anyhow!("Expected subcommand"))?;
    let plugin_store = PluginStore::default()?;
    match plugin_store.read_plugin_manifest(plugin_name) {
        Ok(manifest) => {
            let spin_version = env!("VERGEN_BUILD_SEMVER");
            if let Err(e) = check_supported_version(&manifest, spin_version) {
                eprintln!("{}", e);
                eprintln!(
                    "Try running `spin plugin upgrade {}` to get the latest version of the plugin.",
                    manifest.name()
                );
                process::exit(1);
            }
        }
        Err(e) => {
            // If a manifest file cannot be found for a plugin with the given name
            // error that the command does not exist.
            if let Some(io_e) = e.downcast_ref::<std::io::Error>() {
                if io_e.kind() == std::io::ErrorKind::NotFound {
                    eprintln!("Unknown command.");
                    app.clone().print_help()?;
                    process::exit(1);
                }
            }
            return Err(e);
        }
    }

    let mut command = Command::new(plugin_store.installed_binary_path(plugin_name));
    if args.len() > 1 {
        command.args(&args[1..]);
    }
    command.envs(get_env_vars_map()?);
    log::info!("Executing command {:?}", command);
    // Allow user to interact with stdio/stdout of child process
    let status = command.status().await?;
    log::info!("Exiting process with {}", status);
    if !status.success() {
        match status.code() {
            Some(code) => process::exit(code),
            _ => process::exit(1),
        }
    }
    Ok(())
}

fn get_env_vars_map() -> Result<HashMap<String, String>> {
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
    ]
    .into_iter()
    .collect();
    Ok(map)
}
