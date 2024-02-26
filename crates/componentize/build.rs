use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let adapters_dir = Path::new("adapters");
    std::fs::create_dir_all(out_dir.join("wasm32-unknown-unknown/release")).unwrap();

    println!("cargo:rerun-if-changed=adapters/wasi_snapshot_preview1.spin.wasm");
    fs::copy(
        adapters_dir.join("wasi_snapshot_preview1.spin.wasm"),
        out_dir.join("wasm32-unknown-unknown/release/wasi_snapshot_preview1_spin.wasm"),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=adapters/wasi_snapshot_preview1.reactor.wasm");
    fs::copy(
        adapters_dir.join("wasi_snapshot_preview1.reactor.wasm"),
        out_dir.join("wasm32-unknown-unknown/release/wasi_snapshot_preview1_upstream.wasm"),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=adapters/wasi_snapshot_preview1.command.wasm");
    fs::copy(
        adapters_dir.join("wasi_snapshot_preview1.command.wasm"),
        out_dir.join("wasm32-unknown-unknown/release/wasi_snapshot_preview1_command.wasm"),
    )
    .unwrap();
}
