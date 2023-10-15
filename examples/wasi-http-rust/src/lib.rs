use spin_sdk::http::{IntoResponse, Json};
use spin_sdk::wasi_http_component;

#[derive(serde::Deserialize, Debug)]
struct Greeted {
    name: String,
}

/// A simple Spin HTTP component.
#[wasi_http_component]
async fn hello_world(Json(body): Json<Greeted>) -> anyhow::Result<impl IntoResponse> {
    Ok((200, format!("Hello, {}", body.name)))
}
