use std::{
    collections::HashMap,
    env,
    path::Path,
    process::{self, Command},
};

use cargo_target_dep::build_target_dep;

const RUST_HTTP_INTEGRATION_TEST: &str = "tests/http/simple-spin-rust";
const RUST_HTTP_INTEGRATION_ENV_TEST: &str = "tests/http/headers-env-routes-test";
const RUST_HTTP_VAULT_VARIABLES_TEST: &str = "tests/http/vault-variables-test";
const RUST_OUTBOUND_REDIS_INTEGRATION_TEST: &str = "tests/outbound-redis/http-rust-outbound-redis";
const TIMER_TRIGGER_INTEGRATION_TEST: &str = "examples/spin-timer/app-example";
const WASI_HTTP_INTEGRATION_TEST: &str = "examples/wasi-http-rust-streaming-outgoing-body";

fn main() {
    // Extract environment information to be passed to plugins.
    // Git information will be set to defaults if Spin is not
    // built within a Git worktree.
    vergen::EmitBuilder::builder()
        .build_date()
        .build_timestamp()
        .cargo_target_triple()
        .cargo_debug()
        .git_branch()
        .git_commit_date()
        .git_commit_timestamp()
        .git_sha(true)
        .emit()
        .expect("failed to extract build information");

    let build_spin_tests = env::var("BUILD_SPIN_EXAMPLES")
        .map(|v| v == "1")
        .unwrap_or(true);
    println!("cargo:rerun-if-env-changed=BUILD_SPIN_EXAMPLES");

    if !build_spin_tests {
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
    build_wasm_test_program(
        "rust-http-test.wasm",
        "crates/trigger-http/tests/rust-http-test",
    );
    build_wasm_test_program("redis-rust.wasm", "crates/redis/tests/rust");
    build_wasm_test_program("wagi-test.wasm", "crates/trigger-http/tests/wagi-test");

    build_wasm_test_program(
        "spin-http-benchmark.wasm",
        "crates/trigger-http/benches/spin-http-benchmark",
    );
    build_wasm_test_program(
        "wagi-benchmark.wasm",
        "crates/trigger-http/benches/wagi-benchmark",
    );
    build_wasm_test_program("timer_app_example.wasm", "examples/spin-timer/app-example");

    cargo_build(RUST_HTTP_INTEGRATION_TEST);
    cargo_build(RUST_HTTP_INTEGRATION_ENV_TEST);
    cargo_build(RUST_HTTP_VAULT_VARIABLES_TEST);
    cargo_build(RUST_OUTBOUND_REDIS_INTEGRATION_TEST);
    cargo_build(TIMER_TRIGGER_INTEGRATION_TEST);
    cargo_build(WASI_HTTP_INTEGRATION_TEST);
}

fn build_wasm_test_program(name: &'static str, root: &'static str) {
    build_target_dep(root, Path::new("target/test-programs").join(name))
        .release()
        .target("wasm32-wasi")
        .build();
    println!("cargo:rerun-if-changed={root}/Cargo.toml");
    println!("cargo:rerun-if-changed={root}/Cargo.lock");
    println!("cargo:rerun-if-changed={root}/src");
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
        vec![
            "cargo",
            "build",
            "--target",
            "wasm32-wasi",
            "--release",
            // Ensure that even if `CARGO_TARGET_DIR` is set
            // that we're still building into the right dir.
            "--target-dir",
            "./target",
        ],
        Some(dir),
        None,
    );
    println!("cargo:rerun-if-changed={dir}/Cargo.toml");
    println!("cargo:rerun-if-changed={dir}/src");
}

fn run<S: Into<String> + AsRef<std::ffi::OsStr>>(
    args: Vec<S>,
    dir: Option<S>,
    env: Option<HashMap<S, S>>,
) -> process::Output {
    let mut cmd = Command::new(get_os_process());
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::piped());

    let dir = dir.map(Into::into);
    if let Some(dir) = &dir {
        cmd.current_dir(dir);
    };

    if let Some(env) = env {
        for (k, v) in env {
            cmd.env(k, v);
        }
    };

    cmd.arg("-c");
    let c = args
        .into_iter()
        .map(Into::into)
        .collect::<Vec<String>>()
        .join(" ");
    cmd.arg(&c);

    let output = cmd.output().unwrap();
    let exit = output.status;
    if !exit.success() {
        println!("{}", std::str::from_utf8(&output.stderr).unwrap());
        println!("{}", std::str::from_utf8(&output.stdout).unwrap());
        let dir = dir.unwrap_or_else(current_dir);
        panic!("while running the build script, the command '{c}' failed to run in '{dir}'")
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

fn current_dir() -> String {
    std::env::current_dir()
        .map(|d| d.display().to_string())
        .unwrap_or_else(|_| String::from("<CURRENT DIR>"))
}
