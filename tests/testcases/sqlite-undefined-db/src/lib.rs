use anyhow::Result;
use spin_sdk::http_component;

#[http_component]
fn handle_request(_req: http::Request<()>) -> Result<http::Response<()>> {
    // We don't need to do anything here: it should never get called because
    // spin up should fail at SQLite validation.
    Ok(http::Response::builder().status(200).body(())?)
}
