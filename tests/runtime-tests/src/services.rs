use std::{collections::HashMap, path::Path};

mod docker;
mod python;

use anyhow::{bail, Context};

use docker::{DockerService, DockerServiceConfig};
use python::PythonService;

pub fn start_services(test_path: &Path) -> anyhow::Result<Services> {
    let services_config_path = test_path.join("services");
    let children = if services_config_path.exists() {
        let services = std::fs::read_to_string(&services_config_path)
            .context("could not read services file")?;
        let required_services = services.lines().filter_map(|s| {
            let s = s.trim();
            (!s.is_empty()).then_some(s)
        });
        // TODO: make this more robust so that it is not just assumed where the services definitions are
        let service_definitions_path = test_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("services");
        let service_definitions = std::fs::read_dir(&service_definitions_path)?
            .into_iter()
            .map(|d| {
                let d = d?;
                if !d.file_type()?.is_file() {
                    bail!("directories are not allowed in the service definitions directory")
                }
                let file_name = d.file_name();
                let file_name = file_name.to_str().unwrap();
                let (file_name, file_extension) = file_name
                    .find('.')
                    .map(|i| (&file_name[..i], &file_name[i + 1..]))
                    .context("service definition did not have an extension")?;
                Ok((file_name.to_owned(), file_extension.to_owned()))
            })
            .collect::<anyhow::Result<HashMap<_, _>>>()?;
        let mut services = Vec::new();
        for required_service in required_services {
            let service_definition_extension = service_definitions.get(required_service);
            let child: Box<dyn Service> = match service_definition_extension.map(|s| s.as_str()) {
                Some("py") => Box::new(PythonService::start(
                    required_service,
                    &service_definitions_path,
                )?),
                Some("Dockerfile") => {
                    // TODO: get rid of this hardcoding of ports
                    let config = match required_service {
                        "redis" => DockerServiceConfig { port: 6379 },
                        _ => bail!("unsupported service found: {required_service}"),
                    };
                    Box::new(DockerService::start(
                        required_service,
                        &service_definitions_path,
                        config,
                    )?)
                }
                _ => bail!("unsupported service found: {required_service}"),
            };
            services.push(child);
        }
        services
    } else {
        Vec::new()
    };

    Ok(Services { services: children })
}

/// All the services that are running for a test.
pub struct Services {
    services: Vec<Box<dyn Service>>,
}

impl Services {
    pub fn error(&mut self) -> anyhow::Result<()> {
        for service in &mut self.services {
            service.error()?;
        }
        Ok(())
    }
}

impl Drop for Services {
    fn drop(&mut self) {
        for service in &mut self.services {
            service.stop().unwrap();
        }
    }
}

/// An external service a test may depend on.
trait Service {
    /// Check if the service is in an error state.
    fn error(&mut self) -> anyhow::Result<()>;
    /// Stop the service.
    fn stop(&mut self) -> anyhow::Result<()>;
}
