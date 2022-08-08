//! Commands for the Spin CLI.

/// Command for creating bindles.
pub mod bindle;
/// Commands for building Spin applications.
pub mod build;
/// Command for deploying a Spin app to Hippo
pub mod deploy;
/// commands for external subcommands
pub mod external;
/// Command for creating a new application.
pub mod new;
/// Command for adding a plugin to Spin
pub mod plugins;
/// Commands for working with templates.
pub mod templates;
/// Commands for starting the runtime.
pub mod up;
