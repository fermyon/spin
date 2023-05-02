use spin_sdk::{
    config,
    http::{Request, Response},
    http_component,
};

/// This endpoint returns the config value specified by key.
#[http_component]
fn get(req: Request) -> anyhow::Result<Response> {
    let path = req.uri().path();

    if path.contains("dotenv") {
        let val = config::get("dotenv").expect("Failed to acquire dotenv from spin.toml");
        return Ok(http::Response::builder()
            .status(200)
            .body(Some(val.into()))?);
    }
    let val = format!("message: {}", config::get("message")?);
    Ok(http::Response::builder()
        .status(200)
        .body(Some(val.into()))?)
}
