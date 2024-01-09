use std::path::{Path, PathBuf};

use crate::{services::Services, Runtime, TestResult};
use anyhow::Context as _;

/// A callback to create a runtime given a path to a temporary directory and a set of services
pub type RuntimeCreator = dyn Fn(&mut TestEnvironment) -> anyhow::Result<Box<dyn Runtime>>;

/// All the requirements to run a test
pub struct TestEnvironment {
    temp: temp_dir::TempDir,
    services: Services,
    runtime: Option<Box<dyn Runtime>>,
}

impl TestEnvironment {
    /// Spin up a test environment
    pub fn up(config: &TestEnvironmentConfig) -> anyhow::Result<Self> {
        let temp = temp_dir::TempDir::new()
            .context("failed to produce a temporary directory to run the test in")?;
        log::trace!("Temporary directory: {}", temp.path().display());
        let mut services = Services::start(
            config.required_services.iter().map(String::as_str),
            &config.service_definitions,
        )?;
        services.error().context("services have failed")?;
        let mut env = Self {
            temp,
            services,
            runtime: None,
        };
        let runtime = (config.create_runtime)(&mut env)?;
        env.runtime = Some(runtime);
        env.error().context("services have failed")?;
        Ok(env)
    }

    /// Copy a file into the test environment at the given relative path
    pub fn copy_into(&self, from: impl AsRef<Path>, into: impl AsRef<Path>) -> anyhow::Result<()> {
        let from = from.as_ref();
        std::fs::copy(from, self.temp.path().join(into)).with_context(|| {
            format!(
                "failed to copy file '{}' to temporary directory",
                from.display()
            )
        })?;
        Ok(())
    }

    pub fn get_port(&mut self, guest_port: u16) -> anyhow::Result<Option<u16>> {
        self.services.get_port(guest_port)
    }

    /// Write a file into the test environment at the given relative path
    pub fn write_file(
        &self,
        to: impl AsRef<Path>,
        contents: impl AsRef<[u8]>,
    ) -> anyhow::Result<()> {
        std::fs::write(self.temp.path().join(to), contents)?;
        Ok(())
    }

    /// Run test against runtime
    pub fn test(&mut self) -> anyhow::Result<TestResult> {
        self.runtime
            .as_mut()
            .context("runtime was not initialized")?
            .test()
    }

    /// Get the path to test environment
    pub(crate) fn path(&self) -> &Path {
        self.temp.path()
    }

    /// Whether an error has occurred
    fn error(&mut self) -> anyhow::Result<()> {
        self.services.error()?;
        // TODO: also check for runtime errors
        Ok(())
    }
}

/// Configuration for a test environment
pub struct TestEnvironmentConfig {
    /// The services that the test requires
    pub required_services: Vec<String>,
    /// The path to the service definitions
    pub service_definitions: PathBuf,
    /// A callback to create a runtime given a path to a temporary directory
    pub create_runtime: Box<RuntimeCreator>,
}
