use crate::controller::{AppInstance, Controller};
use crate::metadata_extractor::AppMetadata;
use crate::spin;
use crate::utils;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::process::Output;
use std::time::Duration;
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
        mut xargs: Vec<&str>,
    ) -> Result<AppInstance> {
        let appdir = spin::appdir(app_name);

        let mut cmd = vec!["spin", "up"];
        if !xargs.is_empty() {
            cmd.append(&mut xargs);
        }

        let mut address = "".to_string();
        if trigger_type == "http" {
            let port = utils::get_random_port()?;
            address = format!("127.0.0.1:{}", port);
            cmd.append(&mut vec!["--listen", address.as_str()]);
        }

        let mut child = utils::run_async(cmd, Some(&appdir), None);

        if trigger_type == "http" {
            // ensure the server is accepting requests before continuing.
            match utils::wait_tcp(&address, &mut child, "spin").await {
                Ok(_) => {}
                Err(_) => {
                    let stdout = child.stdout.take().expect("stdout handle not found");
                    let stdout_stream = BufReader::new(stdout);
                    let stdout_logs =
                        utils::get_output_from_stdout(Some(stdout_stream), Duration::from_secs(2))
                            .await?;

                    let stderr = child.stderr.take().expect("stderr handle not found");
                    let stderr_stream = BufReader::new(stderr);
                    let stderr_logs =
                        utils::get_output_from_stderr(Some(stderr_stream), Duration::from_secs(2))
                            .await?;
                    return Err(anyhow!(
                        "error running spin up.\nstdout {:?}\nstderr: {:?}\n",
                        stdout_logs,
                        stderr_logs
                    ));
                }
            }
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

        Ok(AppInstance::new_with_process_and_logs_stream(
            AppMetadata {
                name: app_name.to_string(),
                base: format!("http://{}", address),
                app_routes: vec![],
                version: "".to_string(),
            },
            Some(child),
            Some(stdout_stream),
            Some(stderr_stream),
        ))
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
