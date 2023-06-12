use anyhow::{ensure, Result};
use spin_sdk::{
    config,
    http::{Request, Response},
    http_component,
};

#[http_component]
fn handle_request(req: Request) -> Result<Response> {
    let query = req
        .uri()
        .query()
        .expect("Should have a password query string");
    let query: std::collections::HashMap<String, String> = serde_qs::from_str(query)?;
    let provided_password = query
        .get("password")
        .expect("Should have a password query string");
    let expected_password = config::get("password")?;

    ensure!(
        provided_password == &expected_password,
        "password must match expected"
    );

    Ok(http::Response::builder().status(200).body(None)?)
}
