use std::process::Command;

fn main() {
    let pre = env!("CARGO_PKG_VERSION_PRE");
    let pre = if pre.is_empty() {
        String::new()
    } else {
        format!("-{pre}")
    };

    let commit = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or(String::from("unknown"));

    let commit = commit.trim();

    println!(
        "cargo:rustc-env=SDK_VERSION={}-{}{pre}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
    );
    println!("cargo:rustc-env=SDK_COMMIT={commit}");
}
