//! Commands for the Spin CLI.

/// Command for creating bindles.
pub mod bindle;
/// Commands for building Spin applications.
pub mod build;
/// Command for deploying a Spin app to Hippo
pub mod deploy;
/// Command for creating a new application.
pub mod new;
/// Commands for working with templates.
pub mod templates;
/// Commands for starting the runtime.
pub mod up;

/// Trigger executor commands.
pub mod trigger_http;
pub mod trigger_redis;

pub(crate) mod trigger;
