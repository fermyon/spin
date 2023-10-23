use spin_sdk::http::{IntoResponse, Json, Response};
use spin_sdk::http_component;

#[derive(serde::Deserialize, Debug)]
struct Greeted {
    name: String,
}

/// A simple Spin HTTP component.
#[http_component]
async fn hello_world(req: http::Request<Json<Greeted>>) -> anyhow::Result<impl IntoResponse> {
    Ok(Response::new(200, format!("Hello, {}", req.body().name)))
}
