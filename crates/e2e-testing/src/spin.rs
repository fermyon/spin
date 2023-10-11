use crate::utils;
use anyhow::Result;
use std::path::PathBuf;
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

    let _lock = INSTALLING_TEMPLATES_MUTEX.lock().unwrap();
    let result = utils::run(&cmd, None, None)?;
    utils::assert_success(&result);

    Ok(result)
}

pub fn new_app<'a>(
    template_name: &'a str,
    app_name: &'a str,
    mut args: Vec<&'a str>,
) -> Result<Output> {
    let basedir = utils::testcases_base_dir();
    let mut cmd = vec![
        "spin",
        "new",
        app_name,
        "-t",
        template_name,
        "--accept-defaults",
    ];
    if !args.is_empty() {
        cmd.append(&mut args);
    }

    let output = utils::run(&cmd, Some(&basedir), None)?;
    utils::assert_success(&output);
    Ok(output)
}

pub fn install_plugins(plugins: Vec<&str>) -> Result<Output> {
    // lock mutex to ensure one install_plugins runs at a time
    let _lock = INSTALLING_PLUGINS_MUTEX.lock().unwrap();

    let mut output = utils::run(&["spin", "plugin", "update"], None, None)?;
    utils::assert_success(&output);

    for plugin in plugins {
        output = utils::run(&["spin", "plugin", "install", plugin, "--yes"], None, None)?;
        utils::assert_success(&output);
    }

    Ok(output)
}

pub fn build_app(appname: &str) -> Result<Output> {
    let appdir = appdir(appname);
    utils::run(&["spin", "build"], Some(&appdir), None)
}

pub fn appdir(appname: &str) -> PathBuf {
    utils::testcases_base_dir().join(appname)
}

#[cfg(target_family = "unix")]
pub async fn stop_app_process(process: &mut tokio::process::Child) -> Result<(), anyhow::Error> {
    let pid = process.id().unwrap();
    // println!("stopping app with pid {}", pid);
    let pid = Pid::from_raw(pid as i32);
    kill(pid, Signal::SIGINT).map_err(anyhow::Error::msg)
}

#[cfg(target_family = "windows")]
pub async fn stop_app_process(process: &mut tokio::process::Child) -> Result<(), anyhow::Error> {
    // stop the app at the end of testcase
    let _ = &mut process.kill().await.map_err(anyhow::Error::msg);

    match process.wait().await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::Error::msg(e)),
    }
}

pub fn registry_push(appname: &str, registry_app_url: &str) -> Result<Output> {
    let appdir = appdir(appname);
    let output = utils::run(
        &["spin", "registry", "push", registry_app_url, "--insecure"],
        Some(&appdir),
        None,
    )?;
    utils::assert_success(&output);
    Ok(output)
}

// use docker login until https://github.com/fermyon/spin/issues/1211
pub fn registry_login(registry_url: &str, username: &str, password: &str) -> Result<Output> {
    let output = utils::run(
        &[
            "spin",
            "registry",
            "login",
            "-u",
            username,
            "-p",
            password,
            registry_url,
        ],
        None,
        None,
    )?;
    utils::assert_success(&output);
    Ok(output)
}

pub fn version() -> Result<String> {
    let output = utils::run(&["spin", "--version"], None, None)?;
    utils::assert_success(&output);
    Ok(std::str::from_utf8(&output.stdout)?.trim().to_string())
}

pub fn which_spin() -> Result<String> {
    let output = utils::run(&["which", "spin"], None, None)?;
    utils::assert_success(&output);
    Ok(std::str::from_utf8(&output.stdout)?.trim().to_string())
}
