use crate::spin::Spin;
use crate::test_environment::{RuntimeCreator, TestEnvironment, TestEnvironmentConfig};
use crate::{OnTestError, TestResult};
use anyhow::Context;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

/// Configuration for a runtime test
pub struct RuntimeTestConfig {
    pub test_path: PathBuf,
    pub spin_binary: PathBuf,
    pub on_error: OnTestError,
}

/// A single runtime test
pub struct RuntimeTest {
    test_path: PathBuf,
    on_error: OnTestError,
    env: TestEnvironment,
}

impl RuntimeTest {
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
        let env_config = environment_config(
            &config.test_path,
            Box::new(move |env| {
                copy_manifest(&test_path_clone, env)?;
                Ok(Box::new(Spin::start(&spin_binary, env.path())?) as _)
            }),
        )?;
        let env = TestEnvironment::up(&env_config)?;
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
        let response = match self.env.test() {
            Ok(r) => r,
            Err(e) => {
                error!(on_error, "failed to run test: {e}")
            }
        };
        let error_file = self.test_path.join("error.txt");
        match response {
            TestResult::Pass if !error_file.exists() => log::info!("Test passed!"),
            TestResult::Pass => {
                let expected = match std::fs::read_to_string(&error_file) {
                    Ok(e) => e,
                    Err(e) => error!(on_error, "failed to read error.txt file: {}", e),
                };
                error!(
                    on_error,
                    "Test passed but should have failed with error: {expected}"
                )
            }
            TestResult::Fail(e, extra) if error_file.exists() => {
                let expected = match std::fs::read_to_string(&error_file) {
                    Ok(e) => e,
                    Err(e) => error!(on_error, "failed to read error.txt file: {e}"),
                };
                if e.contains(&expected) {
                    log::info!("Test passed!");
                } else {
                    error!(
                    on_error,
                    "Test errored but not in the expected way.\n\texpected: {expected}\n\tgot: {e}\n\n{extra}",
                )
                }
            }
            TestResult::Fail(e, extra) => {
                error!(
                    on_error,
                    "Test '{}' errored: {e}\n{extra}",
                    self.test_path.display()
                );
            }
            TestResult::RuntimeError(extra) => {
                error!(
                    on_error,
                    "Test '{}' failed fatally: {extra}",
                    self.test_path.display()
                );
            }
        }
        if let OnTestError::Log = on_error {
            println!("'{}' passed", self.test_path.display())
        }
    }
}

/// Start the services that a run test requires.
fn environment_config(
    test_path: &Path,
    create_runtime: Box<RuntimeCreator>,
) -> anyhow::Result<TestEnvironmentConfig> {
    let required_services = required_services(test_path)?;

    // TODO: make this more robust so that it is not just assumed that the services definitions are
    // located at ../../services relative to the test path
    let service_definitions = test_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("services");
    Ok(TestEnvironmentConfig {
        required_services,
        service_definitions,
        create_runtime,
    })
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

static TEMPLATE: OnceLock<regex::Regex> = OnceLock::new();
/// Copies the test dir's manifest file into the temporary directory
///
/// Replaces template variables in the manifest file with components from the components path.
fn copy_manifest(test_dir: &Path, env: &mut TestEnvironment) -> anyhow::Result<()> {
    let manifest_path = test_dir.join("spin.toml");
    let mut manifest = std::fs::read_to_string(manifest_path).with_context(|| {
        format!(
            "no spin.toml manifest found in test directory {}",
            test_dir.display()
        )
    })?;
    let regex = TEMPLATE.get_or_init(|| regex::Regex::new(r"%\{(.*?)\}").unwrap());
    while let Some(captures) = regex.captures(&manifest) {
        let (Some(full), Some(capture)) = (captures.get(0), captures.get(1)) else {
            continue;
        };
        let template = capture.as_str();
        let (template_key, template_value) = template.split_once('=').with_context(|| {
            format!("invalid template '{template}'(template should be in the form $KEY=$VALUE)")
        })?;
        let replacement = match template_key.trim() {
            "source" => {
                let path = std::path::PathBuf::from(
                    test_components::path(template_value)
                        .with_context(|| format!("no such component '{template_value}'"))?,
                );
                let wasm_name = path.file_name().unwrap().to_str().unwrap();
                env.copy_into(&path, wasm_name)?;
                wasm_name.to_owned()
            }
            "port" => {
                let guest_port = template_value
                    .parse()
                    .with_context(|| format!("failed to parse '{template_value}' as port"))?;
                let port = env
                    .get_port(guest_port)?
                    .with_context(|| format!("no port {guest_port} exposed by any service"))?;
                port.to_string()
            }
            _ => {
                anyhow::bail!("unknown template key: {template_key}");
            }
        };
        manifest.replace_range(full.range(), &replacement);
    }
    env.write_file("spin.toml", manifest)
        .context("failed to copy spin.toml manifest to temporary directory")?;
    Ok(())
}
