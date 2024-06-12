use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;

/// A simple Spin HTTP component.
#[http_component]
fn handle_smoke_app(_req: Request) -> anyhow::Result<impl IntoResponse> {
    let var_val = spin_sdk::variables::get("other")?;
    let kv_val = {
        let store = spin_sdk::key_value::Store::open_default()?;
        store.set("k", b"v")?;
        store.get("k")?
    };
    let body = format!("Test response\nVariable: {var_val}\nKV: {kv_val:?}");
    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body(body)
        .build())
}
