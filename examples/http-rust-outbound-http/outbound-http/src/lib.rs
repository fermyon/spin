use anyhow::Result;
use spin_sdk::{
    http::{IntoResponse, Request, Response},
    http_component,
};

/// Send an HTTP request and return the response.
#[http_component]
async fn send_outbound(_req: Request) -> Result<impl IntoResponse> {
    let resp: Response = spin_sdk::http::send(Request::get(
        "https://random-data-api.fermyon.app/animals/json",
    ))
    .await?;
    let resp = resp
        .into_builder()
        .header("spin-component", "rust-outbound-http")
        .build();
    println!("{resp:?}");
    Ok(resp)
}
