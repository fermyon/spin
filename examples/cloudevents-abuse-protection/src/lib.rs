use anyhow::Result;
use spin_sdk::{
    http::{Request, Response},
    http_component,
};
use cloudevents::{Event, AttributesReader};


/// A spin component that can be used to test the event-grid-validation.
/// For more info, see here: https://github.com/cloudevents/spec/blob/v1.0/http-webhook.md#42-validation-response
#[http_component]
fn validation(req: Request) -> Result<Response> {
    println!("{:?}", req);
    if req.method() == http::Method::OPTIONS {
        println!("received validation request");    
        // if let Some(callback) = req.headers().iter().find(|(k, _)| *k == "webhook-request-callback") {
        //     let uri = callback.1.to_str().unwrap();
        //     spin_sdk::outbound_http::send_request(
        //         http::Request::builder()
        //             .header("webhook-response-origin", "eventgrid.azure.net")
        //             .header("webhook-response-callback", uri)
        //             .header("webhook-response-rate", "120")
        //             .method("GET")
        //             .uri(uri)
        //             .body(None)?,
        //     )?;
        // }
        let res = http::Response::builder()
            .status(http::StatusCode::OK)
            .header("webhook-response-origin", "eventgrid.azure.net")
            .header("webhook-response-rate", "120")
            .body(None)?;
        Ok(res)
    } else {
        println!("received event");
        let msg = req.body().as_ref();
        if let Some(msg) = msg {
            let event: Event = serde_json::from_slice(msg)?;
            println!("event: {:?}", event);
            println!("event source: {}", event.source());
            println!("event id: {}", event.id());
            println!("event type: {}", event.ty());
            Ok(http::Response::builder()
            .status(200)
            .body(None)?)
        } else {
            Ok(http::Response::builder()
            .status(400)
            .body(None)?)
        }
    } 
}
