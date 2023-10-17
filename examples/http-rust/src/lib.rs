use spin_sdk::wasi_http::{IntoResponse, Json, Response};
use spin_sdk::wasi_http_component;

#[derive(serde::Deserialize, Debug)]
struct Greeted {
    name: String,
}

/// A simple Spin HTTP component.
#[wasi_http_component]
fn hello_world(req: http::Request<Json<Greeted>>) -> anyhow::Result<impl IntoResponse> {
    Ok(Response::new(200, format!("Hello, {}", req.body().name)))
}
