mod git;
mod lookup;
pub mod manager;
pub mod manifest;
mod prompt;
mod store;
pub use lookup::{fetch_plugins_repo, PluginLookup, PLUGIN_NOT_FOUND_ERROR_MSG};
pub use prompt::prompt_confirm_install;
pub use store::PluginStore;

/// List of Spin internal subcommands
pub(crate) const SPIN_INTERNAL_COMMANDS: [&str; 9] = [
    "templates",
    "up",
    "new",
    "bindle",
    "deploy",
    "build",
    "plugin",
    "trigger",
    "external",
];
