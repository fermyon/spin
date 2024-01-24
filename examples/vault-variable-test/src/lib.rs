use anyhow::Result;
use spin_sdk::{
    http::{IntoResponse, Request, Response},
    http_component,
    variables,
};

#[http_component]
fn handle_vault_variable_test(req: Request) -> Result<impl IntoResponse> {
    let password = std::str::from_utf8(req.body().as_ref()).unwrap();
    let expected = variables::get("test_password").expect("could not get variable");
    let response = if expected == password {
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
