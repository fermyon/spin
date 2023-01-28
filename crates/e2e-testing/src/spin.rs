use crate::utils;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Mutex;

#[cfg(target_family = "unix")]
use {
    nix::sys::signal::{kill, Signal},
    nix::unistd::Pid,
};

static INSTALLING_TEMPLATES_MUTEX: Mutex<i32> = Mutex::new(0);
static INSTALLING_PLUGINS_MUTEX: Mutex<i32> = Mutex::new(0);

pub fn template_install(mut args: Vec<&str>) -> Result<Output> {
    let mut cmd = vec!["spin", "templates", "install"];
    cmd.append(&mut args);

    let mut x = INSTALLING_TEMPLATES_MUTEX.lock().unwrap();
    let result = utils::run(cmd, None, None);

    //this ensure we have mutex lock until here
    *x += 1;

    result
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
    // lock mutex to ensure one install_plugins runs at a time
    let mut x = INSTALLING_PLUGINS_MUTEX.lock().unwrap();

    let mut output = utils::run(vec!["spin", "plugin", "update"], None, None)?;

    for plugin in plugins {
        output = utils::run(
            vec!["spin", "plugin", "install", plugin, "--yes"],
            None,
            None,
        )?;
    }

    //this ensure we have mutex lock until here
    *x += 1;

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
