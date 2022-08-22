#![deny(missing_docs)]

//! A library for building Spin components.

use anyhow::{bail, Context, Result};
use path_absolutize::Absolutize;
use spin_loader::local::config::{RawAppManifest, RawComponentManifest};
use std::path::{Path, PathBuf};
use subprocess::{Exec, Redirection};
use tracing::log;

/// If present, run the build command of each component.
pub async fn build(app: RawAppManifest, src: &Path) -> Result<()> {
    let src = src.absolutize()?;

    if (app.components.iter().all(|c| c.build.is_none())) {
        println!("No build command found!");
        return Ok(());
    }

    let results = futures::future::join_all(
        app.components
            .into_iter()
            .map(|c| build_component(c, &src))
            .collect::<Vec<_>>(),
    )
    .await;

    for r in results {
        if r.is_err() {
            bail!(r.err().unwrap());
        }
    }

    println!("Successfully ran the build command for the Spin components.");
    Ok(())
}

/// Run the build command of the component.
async fn build_component(raw: RawComponentManifest, src: impl AsRef<Path>) -> Result<()> {
    match raw.build {
        Some(b) => {
            println!(
                "Executing the build command for component {}: {}",
                raw.id, b.command
            );
            let workdir = construct_workdir(src.as_ref(), b.workdir.as_ref())?;
            if b.workdir.is_some() {
                println!("Working directory: {:?}", workdir);
            }

            let res = Exec::shell(&b.command)
                .cwd(workdir)
                .stdout(Redirection::Pipe)
                .capture()
                .with_context(|| {
                    format!(
                        "Cannot spawn build process '{:?}' for component {}.",
                        &b.command, raw.id
                    )
                })?;

            if !res.stdout_str().is_empty() {
                log::info!("Standard output for component {}", raw.id);
                print!("{}", res.stdout_str());
            }

            if !res.success() {
                bail!(
                    "Build command for component {} failed with status {:?}.",
                    raw.id,
                    res.exit_status
                );
            }

            Ok(())
        }
        _ => Ok(()),
    }
}

/// Constructs the absolute working directory in which to run the build command.
fn construct_workdir(src: impl AsRef<Path>, workdir: Option<impl AsRef<Path>>) -> Result<PathBuf> {
    let mut cwd = src
        .as_ref()
        .parent()
        .context("The application file did not have a parent directory.")?
        .to_path_buf();

    if let Some(workdir) = workdir {
        // Using `Path::has_root` as `is_relative` and `is_absolute` have
        // surprising behavior on Windows, see:
        // https://doc.rust-lang.org/std/path/struct.Path.html#method.is_absolute
        if workdir.as_ref().has_root() {
            bail!("The workdir specified in the application file must be relative.");
        }
        cwd.push(workdir);
        cwd = cwd.absolutize()?.to_path_buf();
    }

    Ok(cwd)
}
