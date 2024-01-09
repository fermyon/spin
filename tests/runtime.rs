/// Run the tests found in `tests/runtime-tests` directory.
mod runtime_tests {
    use std::path::PathBuf;

    // The macro inspects the tests directory and
    // generates individual tests for each one.
    test_codegen_macro::codegen_tests!();

    fn run(test_path: PathBuf) {
        let config = testing_framework::RuntimeTestConfig {
            test_path,
            spin_binary: env!("CARGO_BIN_EXE_spin").into(),
            on_error: testing_framework::OnTestError::Panic,
        };
        testing_framework::RuntimeTest::bootstrap(config)
            .expect("failed to bootstrap runtime tests tests")
            .run();
    }
}
