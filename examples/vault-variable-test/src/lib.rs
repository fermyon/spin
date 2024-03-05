use anyhow::{Context, Result};
use constant_time_eq::constant_time_eq;
use spin_sdk::{
    http::{IntoResponse, Request, Response},
    http_component, variables,
};

#[http_component]
fn handle_vault_variable_test(req: Request) -> Result<impl IntoResponse> {
    let attempt = std::str::from_utf8(req.body()).unwrap();
    let expected = variables::get("token").context("could not get variable")?;
    let response = if constant_time_eq(&expected.into_bytes(), attempt.as_bytes()) {
        "accepted"
    } else {
        "denied"
    };
    let response_json = format!("{{\"authentication\": \"{}\"}}", response);
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(response_json)
        .build())
}
