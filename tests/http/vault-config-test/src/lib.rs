use anyhow::Result;
use spin_sdk::{
    config,
    http::{Request, Response},
    http_component,
};

/// A simple Spin HTTP component.
#[http_component]
fn config_test(_req: Request) -> Result<Response> {
    let password = config::get("password").expect("Failed to acquire password from vault");
    Ok(http::Response::builder()
        .status(200)
        .body(Some(format!("Got password {}", password).into()))?)
}
