use crate::controller::{AppInstance, Controller};
use crate::metadata_extractor::extract_app_metadata_from_logs;
use crate::spin;
use crate::utils;
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Output;
use std::str;

pub struct FermyonCloud {}

pub const NAME: &str = "fermyon-cloud";

/// implements crate::controller::Controller trait
/// to run apps on `Fermyon Cloud` using `spin deploy`
#[async_trait]
impl Controller for FermyonCloud {
    fn name(&self) -> String {
        NAME.to_string()
    }

    fn login(&self) -> Result<()> {
        Ok(())
    }

    fn template_install(&self, args: Vec<&str>) -> Result<Output> {
        spin::template_install(args)
    }

    fn new_app(&self, template_name: &str, app_name: &str) -> Result<Output> {
        spin::new_app(template_name, app_name)
    }

    fn install_plugins(&self, plugins: Vec<&str>) -> Result<Output> {
        spin::install_plugins(plugins)
    }

    fn build_app(&self, app_name: &str) -> Result<Output> {
        spin::build_app(app_name)
    }

    async fn run_app(&self, app_name: &str) -> Result<AppInstance> {
        let appdir: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "testcases", app_name]
            .iter()
            .collect();

        match utils::run(vec!["spin", "deploy"], appdir.to_str(), None) {
            Err(error) => panic!("problem deploying app {:?}", error),
            Ok(result) => {
                let logs = match str::from_utf8(&result.stdout) {
                    Ok(logs) => logs,
                    Err(error) => panic!("problem fetching deploy logs for app {:?}", error),
                };

                let metadata = extract_app_metadata_from_logs(app_name, logs);
                return Ok(AppInstance::new(metadata));
            }
        };
    }
}
