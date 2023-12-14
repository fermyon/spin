use std::{collections::HashMap, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=components");
    println!("cargo:rerun-if-changed=helper");
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let packages = std::fs::read_dir("components")
        .unwrap()
        .filter_map(|e| {
            let dir = e.ok()?;
            let file_type = dir.file_type().ok()?;
            file_type.is_dir().then_some(dir)
        })
        .map(|e| e.file_name().into_string().unwrap())
        .collect::<Vec<_>>();

    let mut generated_code = String::new();
    let mut name_to_path = HashMap::new();
    for package in packages {
        let crate_path = PathBuf::from("components").join(&package);
        let manifest_path = crate_path.join("Cargo.toml");
        let manifest = cargo_toml::Manifest::from_path(&manifest_path).unwrap();

        // Build the test component
        let mut cargo = Command::new("cargo");
        cargo
            .current_dir(crate_path)
            .arg("build")
            .arg("--target=wasm32-wasi")
            .env("RUSTFLAGS", rustflags())
            .env("CARGO_TARGET_DIR", &out_dir);
        eprintln!("running: {cargo:?}");
        let status = cargo.status().unwrap();
        assert!(status.success(), "{status:?}");
        eprintln!("{status:?}");
        let const_name = to_shouty_snake_case(&package);
        let binary_name = manifest.package.unwrap().name.replace('-', "_");
        let wasm = out_dir
            .join("wasm32-wasi")
            .join("debug")
            .join(format!("{binary_name}.wasm"));

        // Generate const with the wasm binary path
        generated_code += &format!("pub const {const_name}: &str = {wasm:?};\n");
        name_to_path.insert(package, wasm);
    }

    // Generate helper function to map package name to binary path
    generated_code.push_str("pub fn path(name: &str) -> Option<&'static str> {\n");
    for (name, path) in name_to_path {
        generated_code.push_str(&format!("    if name == {name:?} {{\n"));
        generated_code.push_str(&format!("        return Some({path:?});\n"));
        generated_code.push_str("    }\n");
    }
    generated_code.push_str("None\n}");
    std::fs::write(out_dir.join("gen.rs"), generated_code).unwrap();
}

fn to_shouty_snake_case(package: &str) -> String {
    package.to_uppercase().replace('-', "_")
}

fn rustflags() -> &'static str {
    match option_env!("RUSTFLAGS") {
        // If we're in CI which is denying warnings then deny warnings to code
        // built here too to keep the tree warning-free.
        Some(s) if s.contains("-D warnings") => "-D warnings",
        _ => "",
    }
}
