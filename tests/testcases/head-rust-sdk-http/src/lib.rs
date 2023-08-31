use anyhow::Result;
use spin_sdk::{
    config,
    http::{Request, Response},
    http_component,
};

#[http_component]
fn hello_world(req: Request) -> Result<Response> {
    let path = req.uri().path();

    if path.contains("test-placement") {
        match std::fs::read_to_string("/test.txt") {
            Ok(txt) => Ok(http::Response::builder()
                .status(200)
                .body(Some(txt.into()))?),
            Err(e) => anyhow::bail!("Error, could not access test.txt: {}", e),
        }
    } else {
        let msg = config::get("message").expect("Failed to acquire message from spin.toml");

        Ok(http::Response::builder()
            .status(200)
            .body(Some(msg.into()))?)
    }
}
