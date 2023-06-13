use crate::build_info::*;
use crate::commands::plugins::{update, Install};
use crate::opts::PLUGIN_OVERRIDE_COMPATIBILITY_CHECK_FLAG;
use anyhow::{anyhow, Result};
use spin_plugins::{error::Error as PluginError, manifest::warn_unsupported_version, PluginStore};
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

const PREDEFINED_EXTERNALS: &[(&str, &str)] = &[(
    "cloud",
    "Commands for publishing applications to the Fermyon Cloud.",
)];

pub fn predefined_externals() -> Vec<(String, String)> {
    PREDEFINED_EXTERNALS
        .iter()
        .map(|(name, desc)| (name.to_string(), desc.to_string()))
        .collect()
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
            if let Err(e) =
                warn_unsupported_version(&manifest, SPIN_VERSION, override_compatibility_check)
            {
                eprintln!("{e}");
                process::exit(1);
            }
        }
        Err(PluginError::NotFound(e)) => {
            if predefined_externals()
                .iter()
                .any(|(name, _)| name == &plugin_name)
            {
                println!("The `{plugin_name}` plugin is required. Installing now.");
                let plugin_installer = Install {
                    name: Some(plugin_name.clone()),
                    yes_to_all: true,
                    local_manifest_src: None,
                    remote_manifest_src: None,
                    override_compatibility_check: false,
                    version: None,
                };
                // Automatically update plugins if the cloud plugin manifest does not exist
                // TODO: remove this eventually once very unlikely to not have updated
                if let Err(e) = plugin_installer.run().await {
                    if let Some(PluginError::NotFound(_)) = e.downcast_ref::<PluginError>() {
                        update().await?;
                    }
                    plugin_installer.run().await?;
                }
            } else {
                tracing::debug!("Tried to resolve {plugin_name} to plugin, got {e}");
                terminal::error!("'{plugin_name}' is not a known Spin command. See spin --help.\n");
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
            let actual_name = undecorate(sc.get_name());
            if levenshtein::levenshtein(&actual_name, target) <= 2 {
                Some(actual_name)
            } else {
                None
            }
        })
        .collect()
}

fn undecorate(decorated_name: &str) -> String {
    match decorated_name.strip_suffix('*') {
        Some(name) => name.to_owned(),
        None => decorated_name.to_owned(),
    }
}

fn get_env_vars_map() -> Result<HashMap<String, String>> {
    let map: HashMap<String, String> = vec![
        ("SPIN_VERSION", SPIN_VERSION),
        ("SPIN_VERSION_MAJOR", SPIN_VERSION_MAJOR),
        ("SPIN_VERSION_MINOR", SPIN_VERSION_MINOR),
        ("SPIN_VERSION_PATCH", SPIN_VERSION_PATCH),
        ("SPIN_VERSION_PRE", SPIN_VERSION_PRE),
        ("SPIN_COMMIT_SHA", SPIN_COMMIT_SHA),
        ("SPIN_COMMIT_DATE", SPIN_COMMIT_DATE),
        ("SPIN_BRANCH", SPIN_BRANCH),
        ("SPIN_BUILD_DATE", SPIN_BUILD_DATE),
        ("SPIN_TARGET_TRIPLE", SPIN_TARGET_TRIPLE),
        ("SPIN_DEBUG", SPIN_DEBUG),
        (
            "SPIN_BIN_PATH",
            env::current_exe()?
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert binary path to string"))?,
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
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
