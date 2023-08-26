use anyhow::Result;
use spin_sdk::{
    http::{Request, Response, Uri},
    http_component,
};
use url::{Position, Url};

/// Send an HTTP request and return the response.
#[http_component]
fn send_outbound(_req: Request) -> Result<Response> {
    let mut res = spin_sdk::outbound_http::send_request(
        http::Request::builder()
            .method("GET")
            .uri("/hello")
            .body(None)?,
    )?;
    res.headers_mut()
        .insert("spin-component", "outbound-http-component".try_into()?);
    println!("{:?}", res);
    Ok(res)
}
