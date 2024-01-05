/// Run the tests found in `tests/runtime-tests` directory.
mod runtime_tests {
    use runtime_tests::{spin::Spin, Config};
    use std::path::PathBuf;

    // The macro inspects the tests directory and
    // generates individual tests for each one.
    test_codegen_macro::codegen_tests!();

    fn run(name: &str) {
        let spin_binary_path: PathBuf = env!("CARGO_BIN_EXE_spin").into();
        let tests_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/runtime-tests/tests");
        let config = Config {
            create_runtime: Box::new(move |temp, services| {
                Ok(Box::new(Spin::start(&spin_binary_path, temp, services)?) as _)
            }),
            tests_path,
            on_error: runtime_tests::OnTestError::Panic,
        };
        let path = config.tests_path.join(name);
        runtime_tests::bootstrap_and_run(&path, &config)
            .expect("failed to bootstrap runtime tests tests");
    }
}
