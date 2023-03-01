use std::process::Command;

fn main() {
    let pre = env!("CARGO_PKG_VERSION_PRE");
    let pre = if pre.is_empty() {
        String::new()
    } else {
        format!("-{pre}")
    };

    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("failed to execute `git`");

    let commit = if output.status.success() {
        String::from_utf8(output.stdout).unwrap()
    } else {
        panic!("`git` failed: {}", String::from_utf8_lossy(&output.stderr));
    };

    let commit = commit.trim();

    println!(
        "cargo:rustc-env=SDK_VERSION={}-{}{pre}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
    );
    println!("cargo:rustc-env=SDK_COMMIT={commit}");
}
