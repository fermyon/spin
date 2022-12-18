use anyhow::Result;
use spin_sdk::{
    config,
    http::{Request, Response},
    http_component,
};

/// A simple Spin HTTP component.
#[http_component]
fn config_test(_req: Request) -> Result<Response> {
    // Ensure we can get a value from Vault
    let password = config::get("password").expect("Failed to acquire password from vault");
    // Ensure we can get a defaulted value
    let greeting = config::get("greeting").expect("Failed to acquire greeting from default");
    Ok(http::Response::builder()
        .status(200)
        .body(Some(format!("{} Got password {}", greeting, password).into()))?)
}
