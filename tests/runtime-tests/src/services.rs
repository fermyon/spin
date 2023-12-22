use std::{collections::HashMap, path::Path};

mod docker;
mod python;

use anyhow::{bail, Context};

use docker::DockerService;
use python::PythonService;

pub fn start_services(test_path: &Path) -> anyhow::Result<Services> {
    let services_config_path = test_path.join("services");
    if !services_config_path.exists() {
        return Ok(Services {
            services: Vec::new(),
        });
    }

    let services_config_file =
        std::fs::read_to_string(&services_config_path).context("could not read services file")?;
    let required_services = services_config_file.lines().filter_map(|s| {
        let s = s.trim();
        (!s.is_empty()).then_some(s)
    });

    // TODO: make this more robust so that it is not just assumed that the services definitions are
    // located at ../../services relative to the test path
    let service_definitions_path = test_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("services");
    let service_definitions = service_definitions(&service_definitions_path)?;
    let mut services = Vec::new();
    for required_service in required_services {
        let service_definition_extension = service_definitions
            .get(required_service)
            .map(|e| e.as_str());
        let service: Box<dyn Service> = match service_definition_extension {
            Some("py") => Box::new(PythonService::start(
                required_service,
                &service_definitions_path,
            )?),
            Some("Dockerfile") => Box::new(DockerService::start(
                required_service,
                &service_definitions_path,
            )?),
            Some(extension) => {
                bail!("service definitions with the '{extension}' extension are not supported")
            }
            None => bail!("no service definition found for '{required_service}'"),
        };
        service.await_ready()?;
        services.push(service);
    }

    Ok(Services { services })
}

/// Get all of the service definitions returning a HashMap of the service name to the extension.
fn service_definitions(service_definitions_path: &Path) -> anyhow::Result<HashMap<String, String>> {
    std::fs::read_dir(service_definitions_path)?
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
        .filter(|r| !matches!( r , Ok((_, extension)) if extension == "lock"))
        .collect()
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

    /// Get the host port that a service exposes a guest port on.
    pub(crate) fn get_port(&self, guest_port: u16) -> anyhow::Result<Option<u16>> {
        let mut result = None;
        for service in &self.services {
            let host_port = service.ports().unwrap().get(&guest_port);
            match result {
                None => result = host_port.copied(),
                Some(_) => {
                    anyhow::bail!("more than one service exposes port {guest_port} to the host");
                }
            }
        }
        Ok(result)
    }
}

impl<'a> IntoIterator for &'a Services {
    type Item = &'a Box<dyn Service>;
    type IntoIter = std::slice::Iter<'a, Box<dyn Service>>;

    fn into_iter(self) -> Self::IntoIter {
        self.services.iter()
    }
}

/// An external service a test may depend on.
pub trait Service {
    /// The name of the service
    fn name(&self) -> &str;

    /// Block until the service is ready.
    fn await_ready(&self) -> anyhow::Result<()>;

    /// Check if the service is in an error state.
    fn error(&mut self) -> anyhow::Result<()>;

    /// Get a mapping of ports that the service exposes.
    fn ports(&self) -> anyhow::Result<&HashMap<u16, u16>>;
}
