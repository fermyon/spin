use std::path::Path;

use cargo_target_dep::build_target_dep;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Enable spin-factors-derive to emit expanded macro output.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-env=SPIN_FACTORS_DERIVE_EXPAND_DIR={out_dir}");

    let root = "tests/smoke-app";
    build_target_dep(root, Path::new("tests/smoke-app/smoke_app.wasm"))
        .release()
        .target("wasm32-wasi")
        .build();
    println!("cargo:rerun-if-changed={root}/Cargo.toml");
    println!("cargo:rerun-if-changed={root}/Cargo.lock");
    println!("cargo:rerun-if-changed={root}/src");
}
