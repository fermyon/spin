#![deny(missing_docs)]

//! A library for building Spin components.

mod interaction;
mod manifest;
mod scripting;

use anyhow::{anyhow, bail, Context, Result};
use spin_loader::local::parent_dir;
use std::path::{Path, PathBuf};
use subprocess::{Exec, Redirection};

use crate::manifest::{BuildAppInfoAnyVersion, RawComponentManifest};

/// If present, run the build command of each component.
pub async fn build(manifest_file: &Path) -> Result<()> {
    let manifest_text = tokio::fs::read_to_string(manifest_file)
        .await
        .with_context(|| format!("Cannot read manifest file from {}", manifest_file.display()))?;
    let app = toml::from_str(&manifest_text).map(BuildAppInfoAnyVersion::into_v1)?;
    let app_dir = parent_dir(manifest_file)?;

    if app.components.iter().all(|c| c.build.is_none()) {
        println!("No build command found!");
        return Ok(());
    }

    let results = app
        .components
        .into_iter()
        .map(|c| (c.clone(), build_component(&c, &app_dir)))
        .collect::<Vec<_>>();

    let mut fail_count = 0;
    let mut checks = vec![];

    for (c, br) in results {
        if let Err(e) = br {
            fail_count += 1;
            if fail_count == 1 {
                // Blank line before first summary line, others kept together
                eprintln!();
            }
            eprintln!("{e:#}");

            let build_dir = ".spinbuild";
            if let Some(build) = &c.build {
                if let Some(check) = &build.check {
                    let check = match &build.workdir {
                        None => app_dir.join(build_dir).join(check),
                        Some(wd) => app_dir.join(wd).join(build_dir).join(check),
                    };
                    if check.exists() {
                        checks.push(check);
                    }
                }
            }
        }
    }

    if !checks.is_empty() {
        let mut engine = rhai::Engine::new();
        scripting::register_functions(&mut engine);
        for check in checks {
            // Because we have to pipe output directly to the console, we can't pass it to the script.
            // The script will have to assume the worst.
            let check_result = engine.run_file(check.clone());
            if let Err(e) = check_result {
                tracing::warn!("Check script error in {check:?}: {e:?}");
            }
        }
        eprintln!(); // Because one of the checks might have printed something and we want to keep it apart from the Rust termination message
    }

    if fail_count > 0 {
        bail!("Build failed for {fail_count} component(s)")
    }

    println!("Successfully ran the build command for the Spin components.");
    Ok(())
}

/// Run the build command of the component.
fn build_component(raw: &RawComponentManifest, app_dir: &Path) -> Result<()> {
    match raw.build.as_ref() {
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
