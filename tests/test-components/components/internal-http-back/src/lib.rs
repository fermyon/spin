use anyhow::anyhow;
use helper::{ensure_ok, ensure_some};
use spin_sdk::{
    http::{IntoResponse, Request},
    http_component,
};

#[http_component]
async fn handle_back(req: Request) -> anyhow::Result<impl IntoResponse> {
    handle_back_impl(req).await.map_err(|e| anyhow!(e))
}

async fn handle_back_impl(req: Request) -> Result<impl IntoResponse, String> {
    let inbound_rel_path = req
        .header("spin-path-info")
        .and_then(|v| v.as_str());
    let inbound_rel_path = ensure_some!(inbound_rel_path);
    let inbound_body = String::from_utf8_lossy(req.body()).to_string();
    let res = ensure_ok!(http::Response::builder()
        .status(200)
        .header("spin-component", "internal-http-back-component")
        .header("back-received-path", inbound_rel_path)
        .header("back-received-method", req.method().to_string())
        .header("back-received-body", inbound_body)
        .body("Response body from back"));
    Ok(res)
}
