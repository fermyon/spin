use anyhow::Result;
use spin_sdk::{
    http::{Request, Response},
    http_component,
};

// This handler does the following:
// - returns all environment variables as headers with an ENV_ prefix
// - returns all request headers as response headers.
#[http_component]
fn handle_http_request(req: Request) -> Result<Response> {
    match append_headers(http::Response::builder(), &req) {
        Err(e) => anyhow::bail!("Unable to append headers to response: {}", e),
        Ok(resp) => Ok(resp.status(200).body(Some("I'm a teapot".into()))?),
    }
}

fn append_headers(
    mut resp: http::response::Builder,
    req: &Request,
) -> Result<http::response::Builder> {
    for (k, v) in std::env::vars() {
        resp = resp.header(format!("ENV_{}", k), v);
    }
    for (k, v) in req.headers().iter() {
        resp = resp.header(k, v);
    }

    Ok(resp)
}
