#[cfg(feature = "e2e-tests")]
#[test]
fn runtime_tests() {
    use runtime_tests::Config;
    use std::path::PathBuf;

    let spin_binary_path = env!("CARGO_BIN_EXE_spin").into();
    let tests_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/runtime-tests/tests");
    let components_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-components");
    let config = Config {
        spin_binary_path,
        tests_path,
        components_path,
        on_error: runtime_tests::OnTestError::Panic,
    };
    runtime_tests::run(config).expect("failed to bootstrap runtime tests tests")
}
