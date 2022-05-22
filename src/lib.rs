pub mod commands;
pub(crate) mod opts;

use anyhow::{anyhow, bail, Result};
use semver::BuildMetadata;use std::path::{Path, PathBuf};

pub(crate) fn app_dir(app_file: impl AsRef<Path>) -> Result<PathBuf> {
    let path_buf = app_file
        .as_ref()
        .parent()
        .ok_or_else(|| {
            anyhow!(
                "Failed to get containing directory for app file '{}'",
                app_file.as_ref().display()
            )
        })?
        .to_owned();
    Ok(path_buf)
}

pub(crate) fn write_failed_msg(bindle_id: &bindle::Id, dest_dir: &Path) -> String {
    format!(
        "Failed to write bindle '{}' to {}",
        bindle_id,
        dest_dir.display()
    )
}

pub(crate) fn parse_buildinfo(buildinfo: &str) -> Result<BuildMetadata> {
    Ok(BuildMetadata::new(buildinfo)?)
}

/// Parse the environment variables passed in `key=value` pairs.
pub(crate) fn parse_env_var(s: &str) -> Result<(String, String)> {
    let parts: Vec<_> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("Environment variable must be of the form `key=value`");
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
}

// /// Append the environment variables passed as options to all components.
// fn append_env(app: &mut Application, env: &[(String, String)]) -> Result<()> {
//     for c in app.components.iter_mut() {
//         for (k, v) in env {
//             c.wasm.environment.insert(k.clone(), v.clone());
//         }
//     }
//     Ok(())
// }
