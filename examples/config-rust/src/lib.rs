use spin_sdk::{
    config,
    http::{Request, Response},
    http_component,
};

/// This endpoint returns the config value specified by key.
#[http_component]
fn get(req: Request) -> anyhow::Result<Response> {
    if req.path_and_query.contains("dotenv") {
        let val = config::get("dotenv").expect("Failed to acquire dotenv from spin.toml");
        return Ok(Response::new(200, val));
    }
    let val = format!("message: {}", config::get("message")?);
    Ok(Response::new(200, val))
}
