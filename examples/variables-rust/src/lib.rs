use spin_sdk::{
    http::{Request, Response},
    http_component, variables,
};

/// This endpoint returns the config value specified by key.
#[http_component]
fn get(req: Request) -> anyhow::Result<Response> {
    if req.path().contains("dotenv") {
        let val = variables::get("dotenv").expect("Failed to acquire dotenv from spin.toml");
        return Ok(Response::new(200, val));
    }
    let val = format!("message: {}", variables::get("message")?);
    Ok(Response::new(200, val))
}
