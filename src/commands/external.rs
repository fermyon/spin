use crate::opts::PLUGIN_OVERRIDE_COMPATIBILITY_CHECK_FLAG;
use anyhow::{anyhow, Result};
use spin_plugins::{error::Error, manifest::warn_unsupported_version, PluginStore};
use std::{collections::HashMap, env, process, str::FromStr};
use tokio::process::Command;
use tracing::log;

enum CloudCommand {
    Login,
    Deploy,
    Cloud,
}

impl FromStr for CloudCommand {
    type Err = ();

    fn from_str(input: &str) -> Result<CloudCommand, Self::Err> {
        match input {
            "login" => Ok(CloudCommand::Login),
            "deploy" => Ok(CloudCommand::Deploy),
            "cloud" => Ok(CloudCommand::Cloud),
            _ => Err(()),
        }
    }
}

impl CloudCommand {
    // Converts a cloud command to be a subcommand of the `cloud` plugin
    fn make_cloud_subcommand(self, cmd: &mut String, args: &mut Vec<String>) {
        match self {
            Self::Deploy | Self::Login => {
                args.insert(0, cmd.to_string());
                *cmd = "cloud".to_string();
            }
            Self::Cloud => (),
        }
    }
}

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
        .split_first_mut()
        .ok_or_else(|| anyhow!("Expected subcommand"))?;
    let mut args = args.to_vec();
    if let Ok(known_plugin) = CloudCommand::from_str(plugin_name) {
        known_plugin.make_cloud_subcommand(plugin_name, &mut args);
    }
    Ok((plugin_name.to_owned(), args, override_compatibility_check))
}

/// Executes a Spin plugin as a subprocess, expecting the first argument to
/// indicate the plugin to execute. Passes all subsequent arguments on to the
/// subprocess.
pub async fn execute_external_subcommand(
    cmd: Vec<String>,
    app: clap::App<'_>,
) -> anyhow::Result<()> {
    let (plugin_name, args, override_compatibility_check) = parse_subcommand(cmd)?;
    let plugin_store = PluginStore::try_default()?;
    match plugin_store.read_plugin_manifest(&plugin_name) {
        Ok(manifest) => {
            let spin_version = env!("VERGEN_BUILD_SEMVER");
            if let Err(e) =
                warn_unsupported_version(&manifest, spin_version, override_compatibility_check)
            {
                eprintln!("{e}");
                process::exit(1);
            }
        }
        Err(Error::NotFound(e)) => {
            if let Ok(CloudCommand::Cloud) = CloudCommand::from_str(&plugin_name) {
                println!("The `cloud` plugin is required. Installing now.");
                let plugin_installer = crate::commands::plugins::Install {
                    name: Some("cloud".to_string()),
                    ..Default::default()
                };
                plugin_installer.run().await?;
            } else {
                tracing::debug!("Tried to resolve {plugin_name} to plugin, got {e}");
                eprintln!("Error: '{plugin_name}' is not a known Spin command. See spin --help.\n");
                print_similar_commands(app, &plugin_name);
                process::exit(2);
            }
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

fn print_similar_commands(app: clap::App, plugin_name: &str) {
    let similar = similar_commands(app, plugin_name);
    match similar.len() {
        0 => (),
        1 => eprintln!("The most similar command is:"),
        _ => eprintln!("The most similar commands are:"),
    }
    for cmd in &similar {
        eprintln!("    {cmd}");
    }
    if !similar.is_empty() {
        eprintln!();
    }
}

fn similar_commands(app: clap::App, target: &str) -> Vec<String> {
    app.get_subcommands()
        .filter_map(|sc| {
            if levenshtein::levenshtein(sc.get_name(), target) <= 2 {
                Some(sc.get_name().to_owned())
            } else {
                None
            }
        })
        .collect()
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
    fn test_cloud_cmds() {
        assert_eq!(
            parse_subcommand(vec!["deploy".to_string()]).unwrap(),
            ("cloud".to_string(), vec!["deploy".to_string()], false)
        );
        assert_eq!(
            parse_subcommand(vec!["login".to_string()]).unwrap(),
            ("cloud".to_string(), vec!["login".to_string()], false)
        );
        assert_eq!(
            parse_subcommand(vec!["cloud".to_string()]).unwrap(),
            ("cloud".to_string(), vec![], false)
        );
    }

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
