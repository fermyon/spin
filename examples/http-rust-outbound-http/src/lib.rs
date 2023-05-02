use spin_sdk::{
    http::{Request, Response},
    http_component,
};

/// Send an HTTP request and return the response.
#[http_component]
fn send_outbound(_req: Request) -> anyhow::Result<Response> {
    let mut res = spin_sdk::outbound_http::send_request(
        http::Request::builder()
            .method("GET")
            .uri("https://some-random-api.ml/facts/dog")
            .body(None)?,
    )?;
    res.headers_mut()
        .insert("spin-component", "rust-outbound-http".try_into()?);
    println!("{:?}", res);
    Ok(res)
}
