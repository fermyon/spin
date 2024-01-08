pub(crate) mod io;
mod services;
pub mod spin;

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::Context;
use services::Services;

/// A callback to create a runtime given a path to a temporary directory and a set of services
pub type RuntimeCreator = dyn Fn(&Path) -> anyhow::Result<Box<dyn Runtime>>;

/// Configuration for the test suite
pub struct Config {
    /// The runtime under test
    pub create_runtime: Box<RuntimeCreator>,
    /// The path to the tests directory which contains all the runtime test definitions
    pub tests_path: PathBuf,
    /// What to do when an individual test fails
    pub on_error: OnTestError,
}

#[derive(Debug, Clone, Copy)]
/// What to do on a test error
pub enum OnTestError {
    Panic,
    Log,
}

/// Run the runtime tests suite.
///
/// Error represents an error in bootstrapping the tests. What happens on individual test failures
/// is controlled by `config`.
pub fn run_all(config: Config) -> anyhow::Result<()> {
    for test in std::fs::read_dir(&config.tests_path).with_context(|| {
        format!(
            "failed to read test directory '{}'",
            config.tests_path.display()
        )
    })? {
        let test = test.context("I/O error reading entry from test directory")?;
        if !test.file_type()?.is_dir() {
            log::debug!(
                "Ignoring non-sub-directory in test directory: {}",
                test.path().display()
            );
            continue;
        }

        bootstrap_and_run(&test.path(), &config)?;
    }
    Ok(())
}

/// Bootstrap and run a test at a path against the given config
pub fn bootstrap_and_run(test_path: &Path, config: &Config) -> anyhow::Result<()> {
    log::info!("Testing: {}", test_path.display());
    let temp = temp_dir::TempDir::new()
        .context("failed to produce a temporary directory to run the test in")?;
    log::trace!("Temporary directory: {}", temp.path().display());
    let mut services = services::start_services(test_path)?;
    copy_manifest(test_path, &temp, &mut services)?;
    services.error().context("services have failed")?;
    let runtime = &mut *(config.create_runtime)(temp.path())?;
    services.error().context("services have failed")?;
    run_test(runtime, test_path, config.on_error);
    Ok(())
}

pub trait Runtime {
    fn test(&mut self) -> anyhow::Result<TestResult>;
}

/// Run an individual test
fn run_test(runtime: &mut dyn Runtime, test_path: &Path, on_error: OnTestError) {
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
    let response = match runtime.test() {
        Ok(r) => r,
        Err(e) => {
            error!(on_error, "failed to run test: {e}")
        }
    };
    let error_file = test_path.join("error.txt");
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
                test_path.display()
            );
        }
        TestResult::RuntimeError(extra) => {
            error!(
                on_error,
                "Test '{}' failed fatally: {extra}",
                test_path.display()
            );
        }
    }
    if let OnTestError::Log = on_error {
        println!("'{}' passed", test_path.display())
    }
}

static TEMPLATE: OnceLock<regex::Regex> = OnceLock::new();
/// Copies the test dir's manifest file into the temporary directory
///
/// Replaces template variables in the manifest file with components from the components path.
fn copy_manifest(
    test_dir: &Path,
    temp: &temp_dir::TempDir,
    services: &mut Services,
) -> anyhow::Result<()> {
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
                std::fs::copy(&path, temp.path().join(wasm_name)).with_context(|| {
                    format!(
                        "failed to copy wasm file '{}' to temporary directory",
                        path.display()
                    )
                })?;
                wasm_name.to_owned()
            }
            "port" => {
                let guest_port = template_value
                    .parse()
                    .with_context(|| format!("failed to parse '{template_value}' as port"))?;
                let port = services
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
    std::fs::write(temp.path().join("spin.toml"), manifest)
        .context("failed to copy spin.toml manifest to temporary directory")?;
    Ok(())
}

#[derive(Debug)]
pub enum TestResult {
    /// The test passed
    Pass,
    /// Wasm errored (the wasm error, additional error info)
    Fail(String, String),
    /// Wasm failed to run (additional error info)
    RuntimeError(String),
}
