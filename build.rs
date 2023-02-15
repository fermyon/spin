use std::{
    collections::HashMap,
    env,
    path::Path,
    process::{self, Command},
};

use cargo_target_dep::build_target_dep;

const RUST_HTTP_INTEGRATION_TEST: &str = "tests/http/simple-spin-rust";
const RUST_HTTP_INTEGRATION_ENV_TEST: &str = "tests/http/headers-env-routes-test";
const RUST_HTTP_INTEGRATION_KEY_VALUE_TEST: &str = "tests/http/key-value";
const RUST_HTTP_VAULT_CONFIG_TEST: &str = "tests/http/vault-config-test";
const RUST_OUTBOUND_REDIS_INTEGRATION_TEST: &str = "tests/outbound-redis/http-rust-outbound-redis";
const RUST_OUTBOUND_PG_INTEGRATION_TEST: &str = "tests/outbound-pg/http-rust-outbound-pg";

fn main() {
    let mut config = vergen::Config::default();
    *config.git_mut().sha_kind_mut() = vergen::ShaKind::Short;
    *config.git_mut().commit_timestamp_kind_mut() = vergen::TimestampKind::DateOnly;
    vergen::vergen(config).expect("failed to extract build information");

    let build_spin_tests = env::var("BUILD_SPIN_EXAMPLES")
        .map(|v| v == "1")
        .unwrap_or(true);

    if !build_spin_tests {
        println!("cargo:rerun-if-env-changed=BUILD_SPIN_EXAMPLES");
        return;
    }

    println!("cargo:rerun-if-changed=build.rs");

    if !has_wasm32_wasi_target() {
        // Current toolchain: e.g. "stable-x86_64-pc-windows-msvc", "1.60-x86_64-pc-windows-msvc"
        let current_toolchain = std::env::var("RUSTUP_TOOLCHAIN").unwrap();
        let current_toolchain = current_toolchain.split_once('-').unwrap().0;

        // Default toolchain: e.g. "stable (default)", "nightly", "1.60-x86_64-pc-windows-msvc"
        let default_toolchain = run(vec!["rustup", "default"], None, None);
        let default_toolchain = std::str::from_utf8(&default_toolchain.stdout).unwrap();
        let default_toolchain = default_toolchain.split(['-', ' ']).next().unwrap();

        let toolchain_override = if current_toolchain != default_toolchain {
            format!(" +{}", current_toolchain)
        } else {
            String::new()
        };

        println!(
            r#"
error: the `wasm32-wasi` target is not installed
    = help: consider downloading the target with `rustup{} target add wasm32-wasi`"#,
            toolchain_override
        );
        process::exit(1);
    }

    std::fs::create_dir_all("target/test-programs").unwrap();

    build_wasm_test_program("core-wasi-test.wasm", "crates/core/tests/core-wasi-test");
    build_wasm_test_program("rust-http-test.wasm", "crates/http/tests/rust-http-test");
    build_wasm_test_program("redis-rust.wasm", "crates/redis/tests/rust");
    build_wasm_test_program("wagi-test.wasm", "crates/http/tests/wagi-test");

    build_wasm_test_program(
        "spin-http-benchmark.wasm",
        "crates/http/benches/spin-http-benchmark",
    );
    build_wasm_test_program("wagi-benchmark.wasm", "crates/http/benches/wagi-benchmark");
    build_wasm_test_program("timer_app_example.wasm", "examples/spin-timer/app-example");

    cargo_build(RUST_HTTP_INTEGRATION_TEST);
    cargo_build(RUST_HTTP_INTEGRATION_ENV_TEST);
    cargo_build(RUST_HTTP_INTEGRATION_KEY_VALUE_TEST);
    cargo_build(RUST_HTTP_VAULT_CONFIG_TEST);
    cargo_build(RUST_OUTBOUND_REDIS_INTEGRATION_TEST);
    cargo_build(RUST_OUTBOUND_PG_INTEGRATION_TEST);
}

fn build_wasm_test_program(name: &'static str, root: &'static str) {
    build_target_dep(root, Path::new("target/test-programs").join(name))
        .release()
        .target("wasm32-wasi")
        .build();
}

fn has_wasm32_wasi_target() -> bool {
    let output = run(vec!["rustup", "target", "list", "--installed"], None, None);
    let output = std::str::from_utf8(&output.stdout).unwrap();
    for line in output.lines() {
        if line == "wasm32-wasi" {
            return true;
        }
    }

    false
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
) -> process::Output {
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

    output
}

fn get_os_process() -> String {
    if cfg!(target_os = "windows") {
        String::from("powershell.exe")
    } else {
        String::from("bash")
    }
}
