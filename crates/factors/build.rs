fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Enable spin-factors-derive to emit expanded macro output.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-env=SPIN_FACTORS_DERIVE_EXPAND_DIR={out_dir}");
}
