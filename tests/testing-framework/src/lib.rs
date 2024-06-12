//! Testing infrastructure

use test_environment::{Runtime, TestEnvironment};

pub mod runtimes;

#[derive(Debug, Clone, Copy)]
/// What to do when a test errors
pub enum OnTestError {
    /// Panic
    Panic,
    /// Log the error to stderr
    Log,
}

/// A test which can be run against a runtime
pub trait Test {
    /// The runtime the test is run against
    type Runtime: Runtime;
    /// The type of error the test can return when the test is in a failure state
    ///
    /// This type is used when the test is actually run but it fails as opposed to the
    /// error state where the test cannot be run at all.
    type Failure;

    /// Run the test against the runtime
    fn test(self, env: &mut TestEnvironment<Self::Runtime>) -> TestResult<Self::Failure>;
}

impl<F, E> Test for F
where
    F: FnOnce(&mut TestEnvironment<runtimes::spin_cli::SpinCli>) -> TestResult<E> + 'static,
{
    type Runtime = runtimes::spin_cli::SpinCli;
    type Failure = E;

    fn test(self, env: &mut TestEnvironment<Self::Runtime>) -> TestResult<Self::Failure> {
        self(env)
    }
}

/// The result of running a test.
///
/// The result has three states:
/// * `Ok(())` - the test ran and passed
/// * `Err(TestError::Failure(_))` - the test ran and failed
/// * `Err(TestError::Fatal(_))` - the test did not run because of an error
pub type TestResult<E> = Result<(), TestError<E>>;

/// An error in a test.
///
/// This type is generic over the `Failure` type (i.e., the type that is returned when the test
/// is actually run and fails).
#[derive(Debug)]
pub enum TestError<E> {
    /// The test was run but failed.
    Failure(E),
    /// The test did not run because of an error.
    Fatal(anyhow::Error),
}

impl<E> From<anyhow::Error> for TestError<E> {
    fn from(e: anyhow::Error) -> Self {
        TestError::Fatal(e)
    }
}

impl std::error::Error for TestError<anyhow::Error> {}

impl std::fmt::Display for TestError<anyhow::Error> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let e = match self {
            TestError::Failure(e) => {
                write!(f, "{e}")?;
                e
            }
            TestError::Fatal(e) => {
                write!(f, "Test failed to run: {}", e)?;
                e
            }
        };
        for cause in e.chain().skip(1) {
            write!(f, "\n  Caused by: {}", cause)?;
        }
        Ok(())
    }
}
