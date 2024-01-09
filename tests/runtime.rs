/// Run the tests found in `tests/runtime-tests` directory.
mod runtime_tests {
    use std::path::PathBuf;

    // The macro inspects the tests directory and
    // generates individual tests for each one.
    test_codegen_macro::codegen_tests!();

    fn run(name: &str) {
        let tests_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/runtime-tests/tests");
        let config = runtime_tests::RuntimeTestConfig {
            test_path: tests_path.join(name),
            spin_binary: env!("CARGO_BIN_EXE_spin").into(),
            on_error: runtime_tests::OnTestError::Panic,
        };
        runtime_tests::RuntimeTest::bootstrap(config)
            .expect("failed to bootstrap runtime tests tests")
            .run();
    }
}
