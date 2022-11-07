pub mod error;
mod git;
pub mod lookup;
pub mod manager;
pub mod manifest;
mod store;
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
