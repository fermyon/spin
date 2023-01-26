use crate::controller::{AppInstance, Controller};
use crate::metadata_extractor::AppMetadata;
use crate::spin;
use crate::utils;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Output;

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
        return spin::template_install(args);
    }

    fn new_app(&self, template_name: &str, app_name: &str) -> Result<Output> {
        return spin::new_app(template_name, app_name);
    }

    fn install_plugins(&self, plugins: Vec<&str>) -> Result<Output> {
        return spin::install_plugins(plugins);
    }

    fn build_app(&self, app_name: &str) -> Result<Output> {
        return spin::build_app(app_name);
    }

    async fn run_app(&self, app_name: &str) -> Result<AppInstance> {
        let appdir = spin::appdir(app_name);

        let port = utils::get_random_port()?;
        let address = format!("127.0.0.1:{}", port);

        let mut child = utils::run_async(
            vec!["spin", "up", "--listen", &address],
            Some(&appdir),
            None,
        );

        // ensure the server is accepting requests before continuing.
        utils::wait_tcp(&address, &mut child, "spin").await?;

        match utils::get_output(&mut child).await {
            Ok(output) => print!("this output is {:?} until here", output),
            Err(error) => panic!("problem deploying app {:?}", error),
        };

        Ok(AppInstance::new_with_process(
            AppMetadata {
                name: app_name.to_string(),
                base: format!("http://{}", address.to_string()),
                app_routes: vec![],
                version: "".to_string(),
            },
            Some(child),
        ))
    }
}
