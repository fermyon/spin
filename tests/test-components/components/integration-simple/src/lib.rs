use anyhow::Result;
use spin_sdk::{http_component, variables};

#[http_component]
fn hello_world(req: http::Request<()>) -> Result<http::Response<String>> {
    let path = req.uri().path();

    if path.contains("test-placement") {
        match std::fs::read_to_string("/test.txt") {
            Ok(txt) => Ok(http::Response::builder().status(200).body(txt)?),
            Err(e) => anyhow::bail!("Error, could not access test.txt: {}", e),
        }
    } else {
        let msg = variables::get("message").expect("Failed to acquire message from spin.toml");

        Ok(http::Response::builder().status(200).body(msg)?)
    }
}
