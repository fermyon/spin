use crate::utils;
use anyhow::Result;
use lockfile::Lockfile;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;
use waitfor::wait_for;

#[cfg(target_family = "unix")]
use {
    nix::sys::signal::{kill, Signal},
    nix::unistd::Pid,
};

const INSTALLING_PLUGINS_LOCK: &str = "/tmp/installing-plugins.lock";

pub fn template_install(mut args: Vec<&str>) -> Result<Output> {
    let mut cmd = vec!["spin", "templates", "install"];
    cmd.append(&mut args);
    utils::run(cmd, None, None)
}

pub fn new_app(template_name: &str, app_name: &str) -> Result<Output> {
    let basedir: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "tests", "testcases"]
        .iter()
        .collect();

    return utils::run(
        vec!["spin", "new", template_name, app_name, "--accept-defaults"],
        basedir.to_str(),
        None,
    );
}

pub fn install_plugins(plugins: Vec<&str>) -> Result<Output> {
    // prevent running multiple `install plugins` at same time,
    // https://github.com/fermyon/spin/issues/997
    wait_for::<_, _, ()>(Duration::from_secs(30), Duration::from_secs(1), || {
        if Path::new(INSTALLING_PLUGINS_LOCK).exists() {
            Ok(None)
        } else {
            Ok(Some("install plugins not running"))
        }
    })
    .unwrap();

    let lockfile = Lockfile::create(INSTALLING_PLUGINS_LOCK).unwrap();
    let mut output = utils::run(vec!["spin", "plugin", "update"], None, None)?;

    for plugin in plugins {
        output = utils::run(
            vec!["spin", "plugin", "install", plugin, "--yes"],
            None,
            None,
        )?;
    }

    lockfile.release()?;
    Ok(output)
}

pub fn build_app(appname: &str) -> Result<Output> {
    let appdir = appdir(appname);
    utils::run(vec!["spin", "build"], Some(&appdir), None)
}

pub fn appdir(appname: &str) -> String {
    let dir = Path::new(utils::testcases_base_dir().as_str()).join(appname);
    dir.into_os_string().into_string().unwrap()
}

#[cfg(target_family = "unix")]
pub async fn stop_app(process: &mut tokio::process::Child) -> Result<(), anyhow::Error> {
    let pid = process.id().unwrap();
    println!("stopping app with pid {}", pid);
    let pid = Pid::from_raw(pid as i32);
    kill(pid, Signal::SIGINT).map_err(anyhow::Error::msg)
}

#[cfg(target_family = "windows")]
pub async fn stop_app(process: &mut tokio::process::Child) -> Result<(), anyhow::Error> {
    // stop the app at the end of testcase
    let _ = &mut process.kill().await.map_err(anyhow::Error::msg);

    match process.wait().await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::Error::msg(e)),
    }
}
