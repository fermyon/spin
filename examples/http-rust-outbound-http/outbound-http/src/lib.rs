use anyhow::Result;
use spin_sdk::{
    http::{IntoResponse, Request},
    http_component,
};

/// Send an HTTP request and return the response.
#[http_component]
fn send_outbound(_req: Request) -> Result<impl IntoResponse> {
    let res = spin_sdk::http::send(
        http::Request::builder()
            .method("GET")
            .uri("https://random-data-api.fermyon.app/animals/json")
            .body(None)?,
    )?;
    let mut res: http::Response<()> = res.try_into()?;
    res.headers_mut()
        .insert("spin-component", "rust-outbound-http".try_into()?);
    println!("{:?}", res);
    Ok(res)
}
