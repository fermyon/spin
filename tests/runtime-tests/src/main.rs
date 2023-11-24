use std::path::PathBuf;

use runtime_tests::{run, Config, OnTestError};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let tests_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let components_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-components");
    let config = Config {
        spin_binary_path: PathBuf::from("spin"),
        tests_path,
        components_path,
        on_error: OnTestError::Log,
    };
    run(config)
}
