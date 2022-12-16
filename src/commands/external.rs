use crate::opts::PLUGIN_OVERRIDE_COMPATIBILITY_CHECK_FLAG;
use anyhow::{anyhow, Result};
use clap::App;
use spin_plugins::{error::Error, manifest::check_supported_version, PluginStore};
use std::{collections::HashMap, env, process};
use tokio::process::Command;
use tracing::log;

fn override_flag() -> String {
    format!("--{}", PLUGIN_OVERRIDE_COMPATIBILITY_CHECK_FLAG)
}

// Returns true if the argument was removed from the list
fn remove_arg(arg: &str, args: &mut Vec<String>) -> bool {
    let contained = args.contains(&arg.to_owned());
    args.retain(|a| a != arg);
    contained
}

// Parses the subcommand to get the plugin name, args, and override compatibility check flag
fn parse_subcommand(mut cmd: Vec<String>) -> anyhow::Result<(String, Vec<String>, bool)> {
    let override_compatibility_check = remove_arg(&override_flag(), &mut cmd);
    let (plugin_name, args) = cmd
        .split_first()
        .ok_or_else(|| anyhow!("Expected subcommand"))?;
    Ok((
        plugin_name.into(),
        args.to_vec(),
        override_compatibility_check,
    ))
}

/// Executes a Spin plugin as a subprocess, expecting the first argument to
/// indicate the plugin to execute. Passes all subsequent arguments on to the
/// subprocess.
pub async fn execute_external_subcommand(cmd: Vec<String>, app: App<'_>) -> anyhow::Result<()> {
    let (plugin_name, args, override_compatibility_check) = parse_subcommand(cmd)?;
    let plugin_store = PluginStore::try_default()?;
    match plugin_store.read_plugin_manifest(&plugin_name) {
        Ok(manifest) => {
            let spin_version = env!("VERGEN_BUILD_SEMVER");
            if let Err(e) =
                check_supported_version(&manifest, spin_version, override_compatibility_check)
            {
                eprintln!("{e}");
                process::exit(1);
            }
        }
        Err(Error::NotFound(e)) => {
            // Manifest file cannot be found for a plugin with the given name.
            eprintln!("Unknown command: {e}\n");
            app.clone().print_help()?;
            process::exit(2);
        }
        Err(e) => return Err(e.into()),
    }

    let mut command = Command::new(plugin_store.installed_binary_path(&plugin_name));
    command.args(args);
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

#[cfg(test)]
mod test {
    use super::{override_flag, parse_subcommand};

    #[test]
    fn test_remove_arg() {
        let override_flag = override_flag();
        let plugin_name = "example";

        let cmd = vec![plugin_name.to_string()];
        assert_eq!(
            parse_subcommand(cmd).unwrap(),
            (plugin_name.to_string(), vec![], false)
        );

        let cmd_with_args = "example arg1 arg2"
            .split(' ')
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        assert_eq!(
            parse_subcommand(cmd_with_args).unwrap(),
            (
                plugin_name.to_string(),
                vec!["arg1".to_string(), "arg2".to_string()],
                false
            )
        );

        let cmd_with_args_override = format!("example arg1 arg2 {}", override_flag)
            .split(' ')
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        assert_eq!(
            parse_subcommand(cmd_with_args_override).unwrap(),
            (
                plugin_name.to_string(),
                vec!["arg1".to_string(), "arg2".to_string()],
                true
            )
        );

        let cmd_with_args_override = format!("example {} arg1 arg2", override_flag)
            .split(' ')
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        assert_eq!(
            parse_subcommand(cmd_with_args_override).unwrap(),
            (
                plugin_name.to_string(),
                vec!["arg1".to_string(), "arg2".to_string()],
                true
            )
        );

        let cmd_with_args_override = format!("{} example arg1 arg2", override_flag)
            .split(' ')
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        assert_eq!(
            parse_subcommand(cmd_with_args_override).unwrap(),
            (
                plugin_name.to_string(),
                vec!["arg1".to_string(), "arg2".to_string()],
                true
            )
        );
    }
}
