pub mod badger;
pub mod error;
mod git;
pub mod lookup;
pub mod manager;
pub mod manifest;
mod store;
pub use store::PluginStore;

/// List of Spin internal subcommands
pub(crate) const SPIN_INTERNAL_COMMANDS: &[&str] = &[
    "templates",
    "up",
    "new",
    "add",
    "deploy",
    "build",
    "plugin",
    "trigger",
    "external",
];
