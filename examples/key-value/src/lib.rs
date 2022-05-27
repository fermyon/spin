use anyhow::{anyhow, Result};
use spin_sdk::{
    http::{Request, Response},
    http_component, key_value,
};

#[http_component]
fn publish(_req: Request) -> Result<Response> {
    key_value::set(&"key", &b"Eureka!"[..])
        .map_err(|e| anyhow!(format!("Error setting key: {:?}", e)))?;

    let value: Vec<u8> = key_value::get(&"key")
        .map_err(|_| anyhow!("Error getting key"))?
        .expect("Key not found");

    Ok(http::Response::builder()
        .status(200)
        .body(Some(value.into()))?)
}
