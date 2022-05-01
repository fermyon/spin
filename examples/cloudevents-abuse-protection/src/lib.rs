use anyhow::Result;
use spin_sdk::{
    http::{Request, Response},
    http_component,
};

/// A spin component that can be used to test the event-grid-validation.
/// For more info, see here: https://github.com/cloudevents/spec/blob/v1.0/http-webhook.md#42-validation-response
#[http_component]
fn validation(req: Request) -> Result<Response> {
    println!("{:?}", req);
    let mut origin: &str = "";
    let mut callback: &str = "";
    let rate = "120";
    req.headers().iter().for_each(|(k, v)| {
        if k == "webhook-request-origin" {
            origin = v.to_str().unwrap();
        }
        if k == "webhook-request-callback" {
            callback = v.to_str().unwrap();
        }
    });
    Ok(http::Response::builder()
        .status(200)
        .header("webhook-request-origin", origin)
        .header("webhook-request-callback", callback)
        .header("webhook-request-rate", rate)
        .body(None)?)
}
