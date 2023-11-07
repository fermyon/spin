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
    "template",
    "templates",
    "up",
    "new",
    "add",
    "login",
    "deploy",
    "build",
    "plugin",
    "plugins",
    "trigger",
    "external",
    "doctor",
    "registry",
    "watch",
    "oci",
];
