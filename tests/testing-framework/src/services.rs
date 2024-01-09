use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

mod docker;
mod python;

use anyhow::{bail, Context};

use docker::DockerService;
use python::PythonService;

/// All the services that are running for a test.
#[derive(Default)]
pub struct Services {
    services: Vec<Box<dyn Service>>,
}

impl Services {
    /// Start all the required services given a path to service definitions
    pub fn start(config: &ServicesConfig) -> anyhow::Result<Self> {
        let mut services = Vec::new();
        for required_service in &config.services {
            let service_definition_extension =
                config.definitions.get(required_service).map(|e| e.as_str());
            let mut service: Box<dyn Service> = match service_definition_extension {
                Some("py") => Box::new(PythonService::start(
                    required_service,
                    &config.definitions_path,
                )?),
                Some("Dockerfile") => Box::new(DockerService::start(
                    required_service,
                    &config.definitions_path,
                )?),
                Some(extension) => {
                    bail!("service definitions with the '{extension}' extension are not supported")
                }
                None => bail!("no service definition found for '{required_service}'"),
            };
            service.ready()?;
            services.push(service);
        }

        Ok(Services { services })
    }

    pub fn healthy(&mut self) -> anyhow::Result<()> {
        for service in &mut self.services {
            service.ready()?;
        }
        Ok(())
    }

    /// Get the host port that a service exposes a guest port on.
    pub(crate) fn get_port(&mut self, guest_port: u16) -> anyhow::Result<Option<u16>> {
        let mut result = None;
        for service in &mut self.services {
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

pub struct ServicesConfig {
    services: Vec<String>,
    definitions_path: PathBuf,
    definitions: HashMap<String, String>,
}

impl ServicesConfig {
    /// Create a new services config given a path to service definitions and a list of services to start.
    pub fn new(definitions: PathBuf, services: Vec<String>) -> anyhow::Result<Self> {
        let service_definitions = service_definitions(&definitions)?;
        Ok(Self {
            services,
            definitions_path: definitions,
            definitions: service_definitions,
        })
    }

    /// Configure no services
    pub fn none() -> Self {
        Self {
            services: Vec::new(),
            definitions_path: PathBuf::new(),
            definitions: HashMap::new(),
        }
    }
}

/// Get all of the service definitions returning a HashMap of the service name to the service definition file extension.
fn service_definitions(service_definitions_path: &Path) -> anyhow::Result<HashMap<String, String>> {
    std::fs::read_dir(service_definitions_path)
        .with_context(|| {
            format!(
                "no service definitions found at '{}'",
                service_definitions_path.display()
            )
        })?
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

/// An external service a test may depend on.
pub trait Service {
    /// The name of the service.
    fn name(&self) -> &str;

    /// Block until the service is ready and error if service is in bad state.
    fn ready(&mut self) -> anyhow::Result<()>;

    /// Get a mapping of ports that the service exposes.
    fn ports(&mut self) -> anyhow::Result<&HashMap<u16, u16>>;
}
