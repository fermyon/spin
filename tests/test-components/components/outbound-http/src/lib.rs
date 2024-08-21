use anyhow::Result;
use spin_sdk::{
    http::{IntoResponse, Request},
    http_component,
};

/// Send an HTTP request and return the response.
#[http_component]
async fn send_outbound(_req: Request) -> Result<impl IntoResponse> {
    let mut res: http::Response<String> = spin_sdk::http::send(
        http::Request::builder()
            .method("GET")
            .uri("/hello")
            .body(())?,
    )
    .await?;
    res.headers_mut()
        .insert("spin-component", "outbound-http-component".try_into()?);
    Ok(res)
}
