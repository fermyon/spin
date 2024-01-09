mod io;
mod runtime_test;
mod services;
mod spin;
mod test_environment;

pub use runtime_test::{RuntimeTest, RuntimeTestConfig};

#[derive(Debug, Clone, Copy)]
/// What to do when a test errors
pub enum OnTestError {
    Panic,
    Log,
}

pub trait Runtime {
    fn test(&mut self) -> anyhow::Result<TestResult>;
}

#[derive(Debug)]
pub enum TestResult {
    /// The test passed
    Pass,
    /// The runtime ran successfully but the app errored (the wasm error, additional error info)
    Fail(String, String),
    /// The runtime failed to run (additional error info)
    RuntimeError(String),
}
