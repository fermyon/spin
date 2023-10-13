use anyhow::Result;
use spin_sdk::{config, http_component};

/// A simple Spin HTTP component.
#[http_component]
fn config_test(_req: http::Request<()>) -> Result<http::Response<String>> {
    // Ensure we can get a value from Vault
    let password = config::get("password").expect("Failed to acquire password from vault");
    // Ensure we can get a defaulted value
    let greeting = config::get("greeting").expect("Failed to acquire greeting from default");
    Ok(http::Response::builder()
        .status(200)
        .body(format!("{} Got password {}", greeting, password).into())?)
}
