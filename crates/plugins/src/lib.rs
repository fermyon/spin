pub mod error;
mod git;
mod lookup;
pub mod manager;
pub mod manifest;
mod store;
pub use lookup::PluginLookup;
pub use store::PluginStore;

/// List of Spin internal subcommands
pub(crate) const SPIN_INTERNAL_COMMANDS: [&str; 10] = [
    "templates",
    "up",
    "new",
    "add",
    "bindle",
    "deploy",
    "build",
    "plugin",
    "trigger",
    "external",
];
