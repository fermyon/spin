//! Commands for the Spin CLI.

/// Command for creating bindles.
pub mod bindle;
/// Commands for building Spin applications.
pub mod build;
/// Command for deploying a Spin app to Hippo
pub mod deploy;
/// Commands for external subcommands (i.e. plugins)
pub mod external;
// Command for logging into the server
#[cfg(feature = "generate-completions")]
/// Command for generating completions.
pub mod generate_completions;
pub mod login;
/// Command for creating a new application.
pub mod new;
/// Commands for working with OCI registries.
pub mod oci;
/// Command for adding a plugin to Spin
pub mod plugins;
/// Commands for working with templates.
pub mod templates;
/// Commands for starting the runtime.
pub mod up;
