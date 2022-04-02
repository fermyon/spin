use std::{
    collections::HashMap,
    path::Path,
    process::{self, Command},
};

use cargo_target_dep::build_target_dep;

const RUST_HTTP_INTEGRATION_TEST: &str = "tests/http/simple-spin-rust";
const RUST_HTTP_INTEGRATION_ENV_TEST: &str = "tests/http/headers-env-routes-test";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    std::fs::create_dir_all("target/test-programs").unwrap();

    build_wasm_test_program("rust-http-test.wasm", "crates/http/tests/rust-http-test");
    build_wasm_test_program("redis-rust.wasm", "crates/redis/tests/rust");
    build_wasm_test_program("wagi-test.wasm", "crates/http/tests/wagi-test");

    build_wasm_test_program(
        "spin-http-benchmark.wasm",
        "crates/http/benches/spin-http-benchmark",
    );
    build_wasm_test_program("wagi-benchmark.wasm", "crates/http/benches/wagi-benchmark");
    build_wasm_test_program("echo.wasm", "examples/spin-timer/example");

    cargo_build(RUST_HTTP_INTEGRATION_TEST);
    cargo_build(RUST_HTTP_INTEGRATION_ENV_TEST);

    let mut config = vergen::Config::default();
    *config.git_mut().sha_kind_mut() = vergen::ShaKind::Short;
    *config.git_mut().commit_timestamp_kind_mut() = vergen::TimestampKind::DateOnly;
    vergen::vergen(config).expect("failed to extract build information");
}

fn build_wasm_test_program(name: &'static str, root: &'static str) {
    build_target_dep(root, Path::new("target/test-programs").join(name))
        .release()
        .target("wasm32-wasi")
        .build();
}

fn cargo_build(dir: &str) {
    run(
        vec!["cargo", "build", "--target", "wasm32-wasi", "--release"],
        Some(dir),
        None,
    );
}

fn run<S: Into<String> + AsRef<std::ffi::OsStr>>(
    args: Vec<S>,
    dir: Option<S>,
    env: Option<HashMap<S, S>>,
) {
    let mut cmd = Command::new(get_os_process());
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::piped());

    if let Some(dir) = dir {
        cmd.current_dir(dir.into());
    };

    if let Some(env) = env {
        for (k, v) in env {
            cmd.env(k, v);
        }
    };

    cmd.arg("-c");
    cmd.arg(
        args.into_iter()
            .map(Into::into)
            .collect::<Vec<String>>()
            .join(" "),
    );

    let output = cmd.output().unwrap();
    let code = output.status.code().unwrap();
    if code != 0 {
        println!("{:#?}", std::str::from_utf8(&output.stderr).unwrap());
        println!("{:#?}", std::str::from_utf8(&output.stdout).unwrap());
        // just fail
        assert_eq!(0, code);
    }
}

fn get_os_process() -> String {
    if cfg!(target_os = "windows") {
        String::from("powershell.exe")
    } else {
        String::from("/bin/bash")
    }
}
