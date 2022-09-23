//! Commands for the Spin CLI.

use std::path::PathBuf;

use clap::Parser;

/// Command for creating bindles.
pub mod bindle;
/// Commands for building Spin applications.
pub mod build;
/// Command for deploying a Spin app to Hippo
pub mod deploy;
/// Commands for external subcommands (i.e. plugins)
pub mod external;
/// Commands for signing in to Hippo
pub mod login;
/// Command for creating a new application.
pub mod new;
/// Command for adding a plugin to Spin
pub mod plugins;
/// Commands for working with templates.
pub mod templates;
/// Commands for starting the runtime.
pub mod up;


#[derive(Parser, Debug)]
pub struct CommonOpts {
    /// Sets a custom configuration directory
    #[clap(short, long, parse(from_os_str), value_name = "SPIN_CONFIG_DIR")]
    pub dir: Option<PathBuf>,
}