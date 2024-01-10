use anyhow::Context;
use std::path::{Path, PathBuf};
use testing_framework::{
    ManifestTemplate, OnTestError, ServicesConfig, Spin, TestEnvironment, TestEnvironmentConfig,
    TestError, TestResult,
};

/// Configuration for a runtime test
pub struct RuntimeTestConfig {
    pub test_path: PathBuf,
    pub spin_binary: PathBuf,
    pub on_error: OnTestError,
}

/// A single runtime test
pub struct RuntimeTest<R> {
    test_path: PathBuf,
    on_error: OnTestError,
    env: TestEnvironment<R>,
}

impl RuntimeTest<Spin> {
    /// Run the runtime tests suite.
    ///
    /// Error represents an error in bootstrapping the tests. What happens on individual test failures
    /// is controlled by `on_error`.
    pub fn run_all(
        tests_path: &Path,
        spin_binary: PathBuf,
        on_error: OnTestError,
    ) -> anyhow::Result<()> {
        for test in std::fs::read_dir(tests_path)
            .with_context(|| format!("failed to read test directory '{}'", tests_path.display()))?
        {
            let test = test.context("I/O error reading entry from test directory")?;
            if !test.file_type()?.is_dir() {
                log::debug!(
                    "Ignoring non-sub-directory in test directory: {}",
                    test.path().display()
                );
                continue;
            }

            let config = RuntimeTestConfig {
                test_path: test.path(),
                spin_binary: spin_binary.clone(),
                on_error,
            };
            RuntimeTest::bootstrap(config)?.run();
        }
        Ok(())
    }

    pub fn bootstrap(config: RuntimeTestConfig) -> anyhow::Result<Self> {
        log::info!("Testing: {}", config.test_path.display());
        let test_path_clone = config.test_path.to_owned();
        let spin_binary = config.spin_binary.clone();
        let preboot = move |env: &mut TestEnvironment<Spin>| {
            copy_manifest(&test_path_clone, env)?;
            Ok(())
        };
        let services_config = services_config(&config)?;
        let env_config = TestEnvironmentConfig::spin(spin_binary, preboot, services_config);
        let env = TestEnvironment::up(env_config)?;
        Ok(Self {
            test_path: config.test_path,
            on_error: config.on_error,
            env,
        })
    }

    /// Run an individual test
    pub fn run(&mut self) {
        let on_error = self.on_error;
        // macro which will look at `on_error` and do the right thing
        macro_rules! error {
            ($on_error:expr, $($arg:tt)*) => {
                match $on_error {
                    OnTestError::Panic => panic!($($arg)*),
                    OnTestError::Log => {
                        eprintln!($($arg)*);
                        return;
                    }
                }
            };
        }
        let response = self.env.test(test);
        let error_file = self.test_path.join("error.txt");
        match response {
            Ok(()) if !error_file.exists() => log::info!("Test passed!"),
            Ok(()) => {
                let expected = match std::fs::read_to_string(&error_file) {
                    Ok(e) => e,
                    Err(e) => error!(on_error, "failed to read error.txt file: {}", e),
                };
                error!(
                    on_error,
                    "Test passed but should have failed with error: {expected}"
                )
            }
            Err(TestError::Failure(RuntimeTestFailure { error, stderr }))
                if error_file.exists() =>
            {
                let expected = match std::fs::read_to_string(&error_file) {
                    Ok(e) => e,
                    Err(e) => error!(on_error, "failed to read error.txt file: {e}"),
                };
                if error.contains(&expected) {
                    log::info!("Test passed!");
                } else {
                    error!(
                    on_error,
                    "Test errored but not in the expected way.\n\texpected: {expected}\n\tgot: {error}\n\nstderr:\n{stderr}",
                )
                }
            }
            Err(TestError::Failure(RuntimeTestFailure { error, stderr })) => {
                error!(
                    on_error,
                    "Test '{}' errored: {error}\nstderr:\n{stderr}",
                    self.test_path.display()
                );
            }
            Err(TestError::Fatal(extra)) => {
                error!(
                    on_error,
                    "Test '{}' failed to run: {extra}",
                    self.test_path.display()
                );
            }
        }
        if let OnTestError::Log = on_error {
            println!("'{}' passed", self.test_path.display())
        }
    }
}

fn services_config(config: &RuntimeTestConfig) -> anyhow::Result<ServicesConfig> {
    let required_services = required_services(&config.test_path)?;
    let services_config = ServicesConfig::new(required_services)?;
    Ok(services_config)
}

/// Get the services that a test requires.
fn required_services(test_path: &Path) -> anyhow::Result<Vec<String>> {
    let services_config_path = test_path.join("services");
    if !services_config_path.exists() {
        return Ok(Vec::new());
    }
    let services_config_file =
        std::fs::read_to_string(&services_config_path).context("could not read services file")?;
    let iter = services_config_file.lines().filter_map(|s| {
        let s = s.trim();
        (!s.is_empty()).then_some(s.to_owned())
    });
    Ok(iter.collect())
}

/// Copies the test dir's manifest file into the temporary directory
///
/// Replaces template variables in the manifest file with components from the components path.
fn copy_manifest<R>(test_dir: &Path, env: &mut TestEnvironment<R>) -> anyhow::Result<()> {
    let manifest_path = test_dir.join("spin.toml");
    let mut manifest = ManifestTemplate::from_file(manifest_path).with_context(|| {
        format!(
            "no spin.toml manifest found in test directory {}",
            test_dir.display()
        )
    })?;
    manifest.substitute(env)?;
    env.write_file("spin.toml", manifest.contents())
        .context("failed to copy spin.toml manifest to temporary directory")?;
    Ok(())
}

fn test(runtime: &mut Spin) -> TestResult<RuntimeTestFailure> {
    let response = runtime.make_http_request(reqwest::Method::GET, "/")?;
    if response.status() == 200 {
        return Ok(());
    }
    if response.status() != 500 {
        return Err(anyhow::anyhow!("Runtime tests are expected to return either either a 200 or a 500, but it returned a {}", response.status()).into());
    }
    let text = response
        .text()
        .context("could not get runtime test HTTP response")?;
    if text.is_empty() {
        let stderr = runtime.stderr();
        return Err(anyhow::anyhow!("Runtime tests are expected to return a response body, but the response body was empty.\nstderr:\n{stderr}").into());
    }

    Err(TestError::Failure(RuntimeTestFailure {
        error: text,
        stderr: runtime.stderr().to_owned(),
    }))
}

/// A runtime test failure
struct RuntimeTestFailure {
    /// The error message returned by the runtime
    error: String,
    /// The runtime's stderr
    stderr: String,
}
