//! Testing infrastructure
//!
//! This crate has a few entry points depending on what you want to do:
//! * `RuntimeTest` - bootstraps and runs a single runtime test
//! * `TestEnvironment` - bootstraps a test environment which can be used by more than just runtime tests

mod io;
mod manifest_template;
mod runtime_test;
mod services;
mod spin;
mod test_environment;

pub use manifest_template::ManifestTemplate;
pub use runtime_test::{RuntimeTest, RuntimeTestConfig};
pub use services::ServicesConfig;
pub use spin::Spin;
pub use test_environment::{TestEnvironment, TestEnvironmentConfig};

#[derive(Debug, Clone, Copy)]
/// What to do when a test errors
pub enum OnTestError {
    Panic,
    Log,
}

pub trait Runtime {
    /// Run the test against the runtime
    fn test(&mut self) -> TestResult;
    /// Return an error if one has occurred
    fn error(&mut self) -> anyhow::Result<()>;
}

pub type TestResult = Result<(), TestError>;

/// An error in a test
#[derive(Debug)]
pub enum TestError {
    /// The test failed (contains both the error from the runtime which can be compared
    /// against and additional error information)
    Failure(String, anyhow::Error),
    /// The runtime or test runner failed in some way. The test itself did not run
    Fatal(anyhow::Error),
}

impl From<anyhow::Error> for TestError {
    fn from(e: anyhow::Error) -> Self {
        TestError::Fatal(e)
    }
}

impl std::error::Error for TestError {}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::Failure(runtime_error, test_error) => {
                write!(f, "Test failed: {}", runtime_error)?;
                if !test_error.to_string().is_empty() {
                    write!(f, "\nAdditional error information: {}", test_error)?;
                }
                Ok(())
            }
            TestError::Fatal(e) => write!(f, "Test failed to run: {}", e),
        }
    }
}
