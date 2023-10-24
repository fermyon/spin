use anyhow::Result;
use spin_sdk::{http::IntoResponse, http_component};

/// Send an HTTP request and return the response.
#[http_component]
async fn send_outbound(_req: http::Request<()>) -> Result<impl IntoResponse> {
    let mut res: http::Response<String> = spin_sdk::http::send(
        http::Request::builder()
            .method("GET")
            .uri("https://random-data-api.fermyon.app/animals/json")
            .body(())?,
    )
    .await?;
    res.headers_mut()
        .insert("spin-component", "rust-outbound-http".try_into()?);
    println!("{:?}", res);
    Ok(res)
}
