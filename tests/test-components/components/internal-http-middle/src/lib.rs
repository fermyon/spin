use anyhow::anyhow;
use helper::{ensure_eq, ensure_ok, ensure_some};
use spin_sdk::{
    http::{IntoResponse, Request},
    http_component,
};

/// Send an HTTP request and return the response.
#[http_component]
async fn handle_middle(req: Request) -> anyhow::Result<impl IntoResponse> {
    handle_middle_impl(req).await.map_err(|e| anyhow!(e))
}

async fn handle_middle_impl(req: Request) -> Result<impl IntoResponse, String> {
    let inbound_rel_path = req
        .header("spin-path-info")
        .and_then(|v| v.as_str());
    let inbound_rel_path = ensure_some!(inbound_rel_path);
    
    let out_req = spin_sdk::http::Request::builder()
        .uri("https://back.spin.internal/hello/from/middle")
        .method(spin_sdk::http::Method::Post)
        .body("body from middle")
        .build();
    let mut res: http::Response<String> = ensure_ok!(spin_sdk::http::send(out_req).await);

    ensure_eq!("/hello/from/middle", ensure_some!(res.headers().get("back-received-path")));
    ensure_eq!("POST", ensure_some!(res.headers().get("back-received-method")));
    ensure_eq!("body from middle", ensure_some!(res.headers().get("back-received-body")));
    ensure_eq!("Response body from back", res.body());

    res.headers_mut()
        .append("spin-component", ensure_ok!("internal-http-middle-component".try_into()));
    res.headers_mut()
        .append("middle-received-path", ensure_ok!(inbound_rel_path.try_into()));
    res.headers_mut()
        .append("middle-received-method", ensure_ok!(req.method().to_string().try_into()));
    Ok(res)
}
