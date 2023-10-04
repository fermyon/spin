use clap::{CommandFactory, FromArgMatches};
use is_terminal::IsTerminal;
use spin_cli::commands::external::execute_external_subcommand;
use spin_cli::commands::external::predefined_externals;
use spin_cli::subprocess::ExitStatusError;
use spin_cli::SpinApp;

#[tokio::main]
async fn main() {
    if let Err(err) = _main().await {
        let code = match err.downcast_ref::<ExitStatusError>() {
            // If we encounter an `ExitStatusError` it means a subprocess has already
            // exited unsuccessfully and thus already printed error messages. No need
            // to print anything additional.
            Some(e) => e.code(),
            // Otherwise we print the error chain.
            None => {
                terminal::error!("{err}");
                print_error_chain(err);
                1
            }
        };

        std::process::exit(code)
    }
}

async fn _main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("watchexec=off".parse()?),
        )
        .with_ansi(std::io::stderr().is_terminal())
        .init();

    let plugin_help_entries = plugin_help_entries();

    let mut cmd = SpinApp::command();
    for plugin in &plugin_help_entries {
        let subcmd = clap::Command::new(plugin.display_text())
            .about(plugin.about.as_str())
            .allow_hyphen_values(true)
            .disable_help_flag(true)
            .arg(
                clap::Arg::new("command")
                    .allow_hyphen_values(true)
                    .multiple_values(true),
            );
        cmd = cmd.subcommand(subcmd);
    }

    if !plugin_help_entries.is_empty() {
        cmd = cmd.after_help("* implemented via plugin");
    }

    let matches = cmd.clone().get_matches();

    if let Some((subcmd, _)) = matches.subcommand() {
        if plugin_help_entries.iter().any(|e| e.name == subcmd) {
            let command = std::env::args().skip(1).collect();
            return execute_external_subcommand(command, cmd).await;
        }
    }

    SpinApp::from_arg_matches(&matches)?.run(cmd).await
}

fn print_error_chain(err: anyhow::Error) {
    if let Some(cause) = err.source() {
        let is_multiple = cause.source().is_some();
        eprintln!("\nCaused by:");
        for (i, err) in err.chain().skip(1).enumerate() {
            if is_multiple {
                eprintln!("{i:>4}: {}", err)
            } else {
                eprintln!("      {}", err)
            }
        }
    }
}

struct PluginHelpEntry {
    name: String,
    about: String,
}

impl PluginHelpEntry {
    fn from_plugin(plugin: &spin_plugins::manifest::PluginManifest) -> Option<Self> {
        if hide_plugin_in_help(plugin) {
            None
        } else {
            Some(Self {
                name: plugin.name(),
                about: plugin.description().unwrap_or_default().to_owned(),
            })
        }
    }

    fn display_text(&self) -> String {
        format!("{}*", self.name)
    }
}

fn plugin_help_entries() -> Vec<PluginHelpEntry> {
    let mut entries = installed_plugin_help_entries();
    for (name, about) in predefined_externals() {
        if !entries.iter().any(|e| e.name == name) {
            entries.push(PluginHelpEntry { name, about });
        }
    }
    entries
}

fn installed_plugin_help_entries() -> Vec<PluginHelpEntry> {
    let Ok(manager) = spin_plugins::manager::PluginManager::try_default() else {
        return vec![];
    };
    let Ok(manifests) = manager.store().installed_manifests() else {
        return vec![];
    };

    manifests
        .iter()
        .filter_map(PluginHelpEntry::from_plugin)
        .collect()
}

fn hide_plugin_in_help(plugin: &spin_plugins::manifest::PluginManifest) -> bool {
    plugin.name().starts_with("trigger-")
}
