use anyhow::Result;
use spin_sdk::{config, http::IntoResponse, http_component};

/// This endpoint returns the config value specified by key.
#[http_component]
fn get(req: http::Request<()>) -> Result<impl IntoResponse> {
    let path = req.uri().path();

    if path.contains("dotenv") {
        let val = config::get("dotenv").expect("Failed to acquire dotenv from spin.toml");
        return Ok(http::Response::builder().status(200).body(val)?);
    }
    let val = format!("message: {}", config::get("message")?);
    Ok(http::Response::builder().status(200).body(val)?)
}
