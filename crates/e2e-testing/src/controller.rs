use crate::metadata_extractor::AppMetadata;
use anyhow::Result;
use async_trait::async_trait;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::process::Output;

/// defines crate::controller::Controller trait
/// this is to enable running same set of tests
/// using `spin up` or `Deploying to Fermyon Cloud`
#[async_trait]
pub trait Controller {
    fn name(&self) -> String;
    fn login(&self) -> Result<()>;
    fn template_install(&self, args: Vec<&str>) -> Result<Output>;
    fn new_app(&self, template_name: &str, app_name: &str) -> Result<Output>;
    fn build_app(&self, app_name: &str) -> Result<Output>;
    fn install_plugins(&self, plugins: Vec<&str>) -> Result<Output>;
    async fn run_app(&self, app_name: &str) -> Result<AppInstance>;
}
/// This represents a running spin app.
/// If it is running using `spin up`, it also has `process` field populated
/// with handle to the `spin up` process
pub struct AppInstance {
    pub metadata: AppMetadata,
    process: Option<tokio::process::Child>,
}

impl AppInstance {
    pub fn new(metadata: AppMetadata) -> AppInstance {
        AppInstance {
            metadata,
            process: None,
        }
    }

    pub fn new_with_process(
        metadata: AppMetadata,
        process: Option<tokio::process::Child>,
    ) -> AppInstance {
        AppInstance { metadata, process }
    }
}

/// if the app is running using `spin up`, we stop the process when test completes
impl Drop for AppInstance {
    fn drop(&mut self) {
        match &self.process {
            None => (),
            Some(process) => {
                let pid = process.id().unwrap();
                println!("stopping app with pid {}", pid);
                let pid = Pid::from_raw(pid as i32);
                if let Err(e) = kill(pid, Signal::SIGINT) {
                    panic!("error when stopping app with pid {}. {:?}", pid, e)
                }
            }
        }
    }
}
