use std::path::{Path, PathBuf};

use crate::{
    services::{Services, ServicesConfig},
    spin::Spin,
    Runtime, Test, TestResult,
};
use anyhow::Context as _;

/// A callback to create a runtime given a path to a temporary directory and a set of services
pub type RuntimeCreator<R> = dyn FnOnce(&mut TestEnvironment<R>) -> anyhow::Result<R>;

/// All the requirements to run a test
pub struct TestEnvironment<R> {
    temp: temp_dir::TempDir,
    services: Services,
    runtime: Option<R>,
}

impl<R: Runtime> TestEnvironment<R> {
    /// Spin up a test environment
    pub fn up(config: TestEnvironmentConfig<R>) -> anyhow::Result<Self> {
        let temp = temp_dir::TempDir::new()
            .context("failed to produce a temporary directory to run the test in")?;
        log::trace!("Temporary directory: {}", temp.path().display());
        let mut services =
            Services::start(&config.services_config).context("failed to start services")?;
        services.healthy().context("services have failed")?;
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

    /// Run test against runtime
    pub fn test<T: Test<Runtime = R>>(&mut self, test: T) -> TestResult<T::Failure> {
        let runtime = self
            .runtime
            .as_mut()
            .context("runtime was not initialized")?;
        test.test(runtime)
    }

    /// Whether an error has occurred
    fn error(&mut self) -> anyhow::Result<()> {
        self.services.healthy()?;
        if let Some(runtime) = &mut self.runtime {
            runtime.error()?;
        }
        Ok(())
    }
}

impl<R> TestEnvironment<R> {
    /// Copy a file into the test environment at the given relative path
    pub fn copy_into(&self, from: impl AsRef<Path>, into: impl AsRef<Path>) -> anyhow::Result<()> {
        fn copy_dir_all(from: &Path, into: &Path) -> anyhow::Result<()> {
            std::fs::create_dir_all(into)?;
            for entry in std::fs::read_dir(from)? {
                let entry = entry?;
                let ty = entry.file_type()?;
                if ty.is_dir() {
                    copy_dir_all(&entry.path(), &into.join(entry.file_name()))?;
                } else {
                    std::fs::copy(entry.path(), into.join(entry.file_name()))?;
                }
            }
            Ok(())
        }
        let from = from.as_ref();
        let into = into.as_ref();
        if from.is_dir() {
            copy_dir_all(from, &self.temp.path().join(into)).with_context(|| {
                format!(
                    "failed to copy directory '{}' to temporary directory",
                    from.display()
                )
            })?;
        } else {
            std::fs::copy(from, self.temp.path().join(into)).with_context(|| {
                format!(
                    "failed to copy file '{}' to temporary directory",
                    from.display()
                )
            })?;
        }
        Ok(())
    }

    /// Get the host port that is mapped to the given guest port
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

    /// Get the path to test environment
    pub(crate) fn path(&self) -> &Path {
        self.temp.path()
    }
}

/// Configuration for a test environment
pub struct TestEnvironmentConfig<R> {
    /// A callback to create a runtime given a path to a temporary directory
    pub create_runtime: Box<RuntimeCreator<R>>,
    /// The services that the test requires
    pub services_config: ServicesConfig,
}

impl TestEnvironmentConfig<Spin> {
    /// Configure a test environment that uses a local Spin as a runtime
    ///
    /// * `spin_binary` - the path to the Spin binary
    /// * `preboot` - a callback that happens after the services have started but before the runtime is
    /// * `test` - a callback that runs the test against the runtime
    /// * `services_config` - the services that the test requires
    pub fn spin(
        spin_binary: PathBuf,
        preboot: impl FnOnce(&mut TestEnvironment<Spin>) -> anyhow::Result<()> + 'static,
        services_config: ServicesConfig,
    ) -> Self {
        Self {
            services_config,
            create_runtime: Box::new(move |env| {
                preboot(env)?;
                Spin::start(&spin_binary, env.path())
            }),
        }
    }
}
