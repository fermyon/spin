use spin_sdk::http::{IntoResponse, Json};
use spin_sdk::wasi_http_component;

#[derive(serde::Deserialize, Debug)]
struct MyBody {
    data: String,
}

/// A simple Spin HTTP component.
#[wasi_http_component]
async fn hello_world(Json(body): Json<MyBody>) -> anyhow::Result<impl IntoResponse> {
    println!("Body data: {}", body.data);
    Ok((200, "Hello, world"))
}
