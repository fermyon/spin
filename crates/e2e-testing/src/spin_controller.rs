use crate::controller::{AppInstance, Controller, ExitedInstance};
use crate::metadata_extractor::AppMetadata;
use crate::spin;
use crate::utils;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Output;
use tokio::io::BufReader;

pub struct SpinUp {}

/// implements crate::controller::Controller trait
/// to run apps using `spin up`
#[async_trait]
impl Controller for SpinUp {
    fn name(&self) -> String {
        "spin-up".to_string()
    }

    fn login(&self) -> Result<()> {
        Ok(())
    }

    fn template_install(&self, args: Vec<&str>) -> Result<Output> {
        spin::template_install(args)
    }

    fn new_app(&self, template_name: &str, app_name: &str, args: Vec<&str>) -> Result<Output> {
        spin::new_app(template_name, app_name, args)
    }

    fn install_plugins(&self, plugins: Vec<&str>) -> Result<Output> {
        spin::install_plugins(plugins)
    }

    fn build_app(&self, app_name: &str) -> Result<Output> {
        spin::build_app(app_name)
    }

    async fn run_app(
        &self,
        app_name: &str,
        trigger_type: &str,
        mut _deploy_args: Vec<&str>,
        mut up_args: Vec<&str>,
        state_dir: &str,
    ) -> Result<Result<AppInstance, ExitedInstance>> {
        let appdir = spin::appdir(app_name);

        let mut cmd = vec!["spin", "up"];
        if !up_args.is_empty() {
            cmd.append(&mut up_args);
        }

        if !up_args.contains(&"--state_dir") {
            cmd.push("--state-dir");
            cmd.push(state_dir);
        }

        let mut address = String::new();
        if trigger_type == "http" {
            let port = utils::get_random_port()?;
            address = format!("127.0.0.1:{}", port);
            cmd.append(&mut vec!["--listen", address.as_str()]);
        }

        let metadata = AppMetadata {
            name: app_name.to_string(),
            base: format!("http://{}", address),
            app_routes: vec![],
            version: "".to_string(),
        };
        let mut child = utils::run_async(&cmd, Some(&appdir), None);

        // if http ensure the server is accepting requests before continuing.
        if trigger_type == "http" && !utils::wait_tcp(&address, &mut child, "spin").await? {
            let output = child
                .wait_with_output()
                .await
                .context("could not get output from running `spin up`")?;
            return Ok(Err(ExitedInstance { output, metadata }));
        }

        let stdout = child
            .stdout
            .take()
            .expect("child did not have a handle to stdout");
        let stdout_stream = BufReader::new(stdout);

        let stderr = child
            .stderr
            .take()
            .expect("child did not have a handle to stderr");
        let stderr_stream = BufReader::new(stderr);

        Ok(Ok(AppInstance::new_with_process_and_logs_stream(
            metadata,
            Some(child),
            Some(stdout_stream),
            Some(stderr_stream),
        )))
    }

    async fn stop_app(
        &self,
        _: Option<&str>,
        process: Option<tokio::process::Child>,
    ) -> Result<()> {
        match process {
            None => Ok(()),
            Some(mut process) => spin::stop_app_process(&mut process).await,
        }
    }
}
