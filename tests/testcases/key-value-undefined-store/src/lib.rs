use anyhow::Result;
use spin_sdk::{
    http::{Request, Response},
    http_component,
};

#[http_component]
fn handle_request(_req: Request) -> Result<Response> {
    // We don't need to do anything here: it should never get called because
    // spin up should fail at K/V validation.
    Ok(http::Response::builder().status(200).body(None)?)
}
