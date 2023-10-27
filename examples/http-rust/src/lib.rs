use spin_sdk::http::{IntoResponse, Response};
use spin_sdk::http_component;

/// A simple Spin HTTP component.
#[http_component]
fn hello_world(_req: http::Request<()>) -> anyhow::Result<impl IntoResponse> {
    Ok(Response::new(200, "Hello, world!"))
}
