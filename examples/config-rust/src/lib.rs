use anyhow::Result;
use spin_sdk::{
    config,
    http::{Request, Response},
    http_component,
};

/// This endpoint returns the config value specified by key.
#[http_component]
fn get(_req: Request) -> Result<Response> {
    let val = format!("message: {}", config::get("message")?);
    Ok(http::Response::builder()
        .status(200)
        .body(Some(val.into()))?)
}
