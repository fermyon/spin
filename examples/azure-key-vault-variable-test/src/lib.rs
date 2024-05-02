use anyhow::Context;
use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::{http_component, variables};

#[http_component]
fn handle_azure_key_vault_variable_test(_req: Request) -> anyhow::Result<impl IntoResponse> {
    let value = variables::get("secret").context("could not get variable")?;

    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body(format!("Loaded secret from Azure Key Vault: {}", value))
        .build())
}
