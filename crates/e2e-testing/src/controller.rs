use crate::metadata_extractor::AppMetadata;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Output;
use tokio::io::BufReader;
use tokio::process::{ChildStderr, ChildStdout};

/// defines crate::controller::Controller trait
/// this is to enable running same set of tests
/// using `spin up` or `Deploying to Fermyon Cloud`
#[async_trait]
pub trait Controller {
    fn name(&self) -> String;
    fn login(&self) -> Result<()>;
    fn template_install(&self, args: Vec<&str>) -> Result<Output>;
    fn new_app(&self, template_name: &str, app_name: &str, args: Vec<&str>) -> Result<Output>;
    fn build_app(&self, app_name: &str) -> Result<Output>;
    fn install_plugins(&self, plugins: Vec<&str>) -> Result<Output>;
    async fn run_app(
        &self,
        app_name: &str,
        trigger_type: &str,
        mut args: Vec<&str>,
    ) -> Result<AppInstance>;
    async fn stop_app(
        &self,
        app_name: Option<&str>,
        process: Option<tokio::process::Child>,
    ) -> Result<()>;
}
/// This represents a running spin app.
/// If it is running using `spin up`, it also has `process` field populated
/// with handle to the `spin up` process
pub struct AppInstance {
    pub metadata: AppMetadata,
    pub process: Option<tokio::process::Child>,
    pub stdout_stream: Option<BufReader<ChildStdout>>,
    pub stderr_stream: Option<BufReader<ChildStderr>>,
}

impl AppInstance {
    pub fn new(metadata: AppMetadata) -> AppInstance {
        AppInstance {
            metadata,
            process: None,
            stdout_stream: None,
            stderr_stream: None,
        }
    }

    pub fn new_with_process(
        metadata: AppMetadata,
        process: Option<tokio::process::Child>,
    ) -> AppInstance {
        AppInstance {
            metadata,
            process,
            stdout_stream: None,
            stderr_stream: None,
        }
    }

    pub fn new_with_process_and_logs_stream(
        metadata: AppMetadata,
        process: Option<tokio::process::Child>,
        stdout_stream: Option<BufReader<ChildStdout>>,
        stderr_stream: Option<BufReader<ChildStderr>>,
    ) -> AppInstance {
        AppInstance {
            metadata,
            process,
            stdout_stream,
            stderr_stream,
        }
    }
}
