use spin_sdk::http::{IntoResponse, Json};
use spin_sdk::http_component;

#[derive(serde::Deserialize, Debug)]
struct Greeted {
    name: String,
}

/// A simple Spin HTTP component.
#[http_component]
fn hello_world(req: http::Request<Json<Greeted>>) -> anyhow::Result<impl IntoResponse> {
    Ok(http::Response::builder()
        .status(200)
        .body(format!("Hello, {}", req.body().name))?)
}
