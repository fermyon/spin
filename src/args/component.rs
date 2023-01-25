use std::path::PathBuf;

use super::temp::TempDir;
use anyhow::{bail, Result};
use clap::Args;

/// Options to pass to components
#[derive(Args, Debug, Clone, Default)]
#[command(next_help_heading = "Component Options")]
pub struct ComponentOptions {
    /// Temporary directory for the static assets of the components.
    #[arg(long, default_value_t = TempDir::default())]
    pub temp: TempDir,
    /// Pass an environment variable (key=value) to all components of the application.
    #[arg(short, long, value_parser = parse_env_var)]
    pub env: Vec<(String, String)>,
}

impl ComponentOptions {
    pub fn working_dir(&self) -> Result<PathBuf> {
        Ok(self.temp.as_path().canonicalize()?)
    }
}

// Parse the environment variables passed in `key=value` pairs.
fn parse_env_var(env: &str) -> Result<(String, String)> {
    if let Some((var, value)) = env.split_once('=') {
        Ok((var.to_owned(), value.to_owned()))
    } else {
        bail!("Environment variable must be of the form `key=value`")
    }
}
