use anyhow::{ensure, Result};
use itertools::sorted;
use spin_sdk::{
    http::{Request, Response},
    http_component,
    key_value::{Error, Store},
};

#[http_component]
fn handle_request(req: Request) -> Result<Response> {
    // We don't need to do anything here: it should never get called because
    // spin up should fail at K/V validation.
    Ok(http::Response::builder().status(200).body(None)?)
}
