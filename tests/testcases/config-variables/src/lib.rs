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
    let expected_password_value = query
        .get("password")
        .expect("Should have a password query string");
    let actual_password_value = config::get("password")?;

    ensure!(
        expected_password_value == &actual_password_value,
        "actual password value from config store '{}' must match expected password value '{}'",
        &actual_password_value,
        expected_password_value
    );

    Ok(http::Response::builder().status(200).body(None)?)
}
