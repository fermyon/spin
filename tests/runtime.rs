/// Run the tests found in `tests/runtime-tests` directory.
mod runtime_tests {
    use std::path::PathBuf;

    // The macro inspects the tests directory and
    // generates individual tests for each one.
    test_codegen_macro::codegen_runtime_tests!(
        ignore: [
            // This test is flaky. Often gets "Connection reset by peer" errors.
            // https://github.com/fermyon/spin/issues/2265
            "outbound-postgres"
        ]
    );

    fn run(test_path: PathBuf) {
        let config = runtime_tests::RuntimeTestConfig {
            test_path,
            runtime_config: (),
            // runtime_config: testing_framework::SpinConfig {
            //     binary_path: env!("CARGO_BIN_EXE_spin").into(),
            // },
            on_error: testing_framework::OnTestError::Panic,
        };
        runtime_tests::RuntimeTest::<testing_framework::InMemorySpin>::bootstrap(config)
            .expect("failed to bootstrap runtime tests tests")
            .run();
    }
}
