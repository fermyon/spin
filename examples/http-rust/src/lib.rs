use spin_sdk::http::{IntoResponse, Json};
use spin_sdk::http_component;

#[derive(serde::Deserialize, Debug)]
struct MyBody {
    data: String,
}

/// A simple Spin HTTP component.
#[http_component]
fn hello_world(Json(body): Json<MyBody>) -> impl IntoResponse {
    println!("Body: {:?}", body.data);
    (200, "Hello, world")
}
