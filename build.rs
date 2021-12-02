use std::{
    collections::HashMap,
    process::{self, Command},
};

const ECHO_WAI: &str = "crates/engine/tests/echo.witx";
const ECHO_RUST: &str = "crates/engine/tests/rust-echo";

const HTTP_WAI: &str = "crates/http/spin_http_v01.wai";
const HTTP_TEST: &str = "crates/http/tests/rust-http-test";

const REDIS_WIT: &str = "crates/redis/wit/spin_redis_trigger_v01.wit";
const REDIS_TEST_RUST: &str = "crates/redis/tests/rust";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rerun-if-changed={}", ECHO_WAI);
    println!("cargo:rerun-if-changed={}/src/lib.rs", ECHO_RUST);

    println!("cargo:rerun-if-changed={}", HTTP_WAI);
    println!("cargo:rerun-if-changed={}/src/lib.rs", HTTP_TEST);

    println!("cargo:rerun-if-changed={}", REDIS_WIT);
    println!("cargo:rerun-if-changed={}/src/lib.rs", REDIS_TEST_RUST);

    cargo_build(ECHO_RUST);
    cargo_build(HTTP_TEST);
    cargo_build(REDIS_TEST_RUST);
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
