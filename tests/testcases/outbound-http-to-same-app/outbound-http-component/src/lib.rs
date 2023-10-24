use anyhow::Result;
use spin_sdk::{http::IntoResponse, http_component};

/// Send an HTTP request and return the response.
#[http_component]
async fn send_outbound(_req: http::Request<()>) -> Result<impl IntoResponse> {
    let mut res: http::Response<String> = spin_sdk::http::send(
        http::Request::builder()
            .method("GET")
            .uri("/test/hello")
            .body(())?,
    )
    .await?;
    res.headers_mut()
        .insert("spin-component", "outbound-http-component".try_into()?);
    println!("{:?}", res);
    Ok(res)
}
