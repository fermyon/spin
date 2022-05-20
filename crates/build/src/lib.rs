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
            if !src.as_ref().starts_with(workdir.as_path()) {
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
        if !workdir.as_ref().is_relative() {
            bail!("The workdir is not relative.");
        }
        cwd.push(workdir);
        cwd = cwd.absolutize()?.to_path_buf();
    }

    Ok(cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_workdir() {
        /// Compares paths with `strip_prefix` instead of `==` to avoid handling
        /// different operating systems separately.
        fn assert_workdir(
            src: impl AsRef<Path>,
            workdir_param: Option<impl AsRef<Path>>,
            expected: impl AsRef<Path>,
        ) {
            let got = construct_workdir(src.as_ref(), workdir_param.as_ref()).unwrap();
            assert_eq!(
                got.strip_prefix(expected.as_ref()),
                Ok(Path::new("")),
                "{:?} != {:?}",
                got,
                expected.as_ref(),
            )
        }

        let src = Path::new("/home/alice/app/spin.toml");
        assert_workdir(src, None::<PathBuf>, "/home/alice/app");
        assert_workdir(src, Some("foo/bar"), "/home/alice/app/foo/bar");
        assert_workdir(src, Some("../other-app"), "/home/alice/other-app");

        assert!(construct_workdir(src, Some("/etc")).is_err());
    }
}
