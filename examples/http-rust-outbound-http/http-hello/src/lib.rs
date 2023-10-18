use anyhow::Result;
use spin_sdk::http_component;

/// A simple Spin HTTP component.
#[http_component]
fn hello_world(_req: http::Request<()>) -> Result<http::Response<&'static str>> {
    Ok(http::Response::builder()
        .status(200)
        .body("Hello, Fermyon!\n")?)
}
