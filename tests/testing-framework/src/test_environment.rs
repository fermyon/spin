use std::path::{Path, PathBuf};

use crate::{
    services::{Services, ServicesConfig},
    spin::{Spin, SpinMode},
    Runtime,
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
    /// Spin up a test environment with a runtime
    pub fn up(config: TestEnvironmentConfig<R>) -> anyhow::Result<Self> {
        let mut env = Self::boot(&config.services_config)?;
        let runtime = (config.create_runtime)(&mut env)?;
        env.start_runtime(runtime)
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
    /// Spin up a test environment without a runtime
    pub fn boot(services: &ServicesConfig) -> anyhow::Result<Self> {
        let temp = temp_dir::TempDir::new()
            .context("failed to produce a temporary directory to run the test in")?;
        log::trace!("Temporary directory: {}", temp.path().display());
        let mut services = Services::start(services).context("failed to start services")?;
        services.healthy().context("services have failed")?;
        Ok(Self {
            temp,
            services,
            runtime: None,
        })
    }

    /// Start the runtime
    pub fn start_runtime<N: Runtime>(self, runtime: N) -> anyhow::Result<TestEnvironment<N>> {
        let mut this = TestEnvironment {
            temp: self.temp,
            services: self.services,
            runtime: Some(runtime),
        };
        this.error().context("testing environment is not healthy")?;
        Ok(this)
    }

    /// Get the services that are running for the test
    pub fn services_mut(&mut self) -> &mut Services {
        &mut self.services
    }

    /// Get the runtime that is running for the test
    pub fn runtime_mut(&mut self) -> &mut R {
        self.runtime
            .as_mut()
            .expect("runtime has not been initialized")
    }

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

    /// Read a file from the test environment at the given relative path
    pub fn read_file(&self, path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
        std::fs::read(self.temp.path().join(path))
    }

    /// Run a command in the test environment
    ///
    /// This blocks until the command has finished running and will error if the command fails
    pub fn run_in(&self, cmd: &mut std::process::Command) -> anyhow::Result<std::process::Output> {
        let output = cmd.current_dir(self.temp.path()).output()?;
        if !output.status.success() {
            anyhow::bail!(
                "'{cmd:?}' failed with status code {:?}\nstdout:\n{}\nstderr:\n{}\n",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(output)
    }

    /// Get the path to test environment
    pub(crate) fn path(&self) -> &Path {
        self.temp.path()
    }
}

/// Configuration for a test environment
pub struct TestEnvironmentConfig<R> {
    /// A callback to create a runtime given a path to a temporary directory
    create_runtime: Box<RuntimeCreator<R>>,
    /// The services that the test requires
    services_config: ServicesConfig,
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
        spin_up_args: impl IntoIterator<Item = String>,
        preboot: impl FnOnce(&mut TestEnvironment<Spin>) -> anyhow::Result<()> + 'static,
        services_config: ServicesConfig,
        mode: SpinMode,
    ) -> Self {
        let spin_up_args = spin_up_args.into_iter().collect();
        Self {
            services_config,
            create_runtime: Box::new(move |env| {
                preboot(env)?;
                Spin::start(&spin_binary, env, spin_up_args, mode)
            }),
        }
    }
}
