use spin_sdk::{http_component, variables};

/// This endpoint returns the config value specified by key.
#[http_component]
fn get(req: http::Request<()>) -> anyhow::Result<http::Response<String>> {
    if req.uri().path().contains("dotenv") {
        let val = variables::get("dotenv").expect("Failed to acquire dotenv from spin.toml");
        return Ok(http::Response::builder().status(200).body(val)?);
    }
    let val = format!("message: {}", variables::get("message")?);
    Ok(http::Response::builder().status(200).body(val)?)
}
