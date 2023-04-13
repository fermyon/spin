#![deny(missing_docs)]

//! A library for building Spin components.

mod manifest;

use anyhow::{anyhow, bail, Context, Result};
use manifest::BuildAppInfoV1;
use spin_loader::local::{
    config::{is_missing_tag_error, VersionTagLoader},
    parent_dir,
};
use std::path::{Path, PathBuf};
use subprocess::{Exec, Redirection};

use crate::manifest::{BuildAppInfoAnyVersion, RawComponentManifest};

/// If present, run the build command of each component.
pub async fn build(manifest_file: &Path) -> Result<()> {
    let manifest_text = tokio::fs::read_to_string(manifest_file)
        .await
        .with_context(|| format!("Cannot read manifest file from {}", manifest_file.display()))?;
    let app = raw_manifest_from_str(&manifest_text).map(BuildAppInfoAnyVersion::into_v1)?;
    let app_dir = parent_dir(manifest_file)?;

    if app.components.iter().all(|c| c.build.is_none()) {
        println!("No build command found!");
        return Ok(());
    }

    app.components
        .into_iter()
        .map(|c| build_component(c, &app_dir))
        .collect::<Result<Vec<_>, _>>()?;

    println!("Successfully ran the build command for the Spin components.");
    Ok(())
}

/// Run the build command of the component.
fn build_component(raw: RawComponentManifest, app_dir: &Path) -> Result<()> {
    match raw.build {
        Some(b) => {
            println!(
                "Executing the build command for component {}: {}",
                raw.id, b.command
            );
            let workdir = construct_workdir(app_dir, b.workdir.as_ref())?;
            if b.workdir.is_some() {
                println!("Working directory: {:?}", workdir);
            }

            let exit_status = Exec::shell(&b.command)
                .cwd(workdir)
                .stdout(Redirection::None)
                .stderr(Redirection::None)
                .stdin(Redirection::None)
                .popen()
                .map_err(|err| {
                    anyhow!(
                        "Cannot spawn build process '{:?}' for component {}: {}",
                        &b.command,
                        raw.id,
                        err
                    )
                })?
                .wait()?;

            if !exit_status.success() {
                bail!(
                    "Build command for component {} failed with status {:?}",
                    raw.id,
                    exit_status,
                );
            }

            Ok(())
        }
        _ => Ok(()),
    }
}

/// Constructs the absolute working directory in which to run the build command.
fn construct_workdir(app_dir: &Path, workdir: Option<impl AsRef<Path>>) -> Result<PathBuf> {
    let mut cwd = app_dir.to_owned();

    if let Some(workdir) = workdir {
        // Using `Path::has_root` as `is_relative` and `is_absolute` have
        // surprising behavior on Windows, see:
        // https://doc.rust-lang.org/std/path/struct.Path.html#method.is_absolute
        if workdir.as_ref().has_root() {
            bail!("The workdir specified in the application file must be relative.");
        }
        cwd.push(workdir);
    }

    Ok(cwd)
}

fn raw_manifest_from_str(buf: &str) -> Result<BuildAppInfoAnyVersion> {
    use serde::Deserialize;
    let tl = toml::from_str(buf);
    let tl = if is_missing_tag_error(&tl) {
        tl.context("Manifest must contain spin_manifest_version with a value of \"1\"")?
    } else {
        tl?
    };

    match tl {
        VersionTagLoader::OldV1 { rest, .. } => {
            let raw = BuildAppInfoV1::deserialize(rest)?;
            Ok(BuildAppInfoAnyVersion::V1(raw))
        }
        VersionTagLoader::NewV1 { rest, .. } => {
            let raw = BuildAppInfoV1::deserialize(rest)?;
            Ok(BuildAppInfoAnyVersion::V1(raw))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_data_root() -> PathBuf {
        let crate_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(crate_dir).join("tests")
    }

    #[tokio::test]
    async fn can_load_even_if_trigger_invalid() {
        let bad_trigger_file = test_data_root().join("bad_trigger.toml");
        build(&bad_trigger_file).await.unwrap();
    }
}
