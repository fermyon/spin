/// Run the tests found in `tests/runtime-tests` directory.
mod runtime_tests {
    use runtime_tests::Config;
    use std::path::PathBuf;

    // The macro inspects the tests directory and
    // generates individual tests for each one.
    test_codegen_macro::codegen_tests!();

    fn run(name: &str) {
        let spin_binary_path = env!("CARGO_BIN_EXE_spin").into();
        let tests_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/runtime-tests/tests");
        let config = Config {
            spin_binary_path,
            tests_path,
            on_error: runtime_tests::OnTestError::Panic,
        };
        let path = config.tests_path.join(name);
        runtime_tests::bootstrap_and_run(&path, &config)
            .expect("failed to bootstrap runtime tests tests");
    }
}
