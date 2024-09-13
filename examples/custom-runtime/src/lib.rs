use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;

mod bindings {
    wit_bindgen::generate!({
        path: "wit"

    });
}

/// A simple Spin HTTP component.
#[http_component]
fn handle_deleteme(req: Request) -> anyhow::Result<impl IntoResponse> {
    let answer = bindings::my_company::my_product::llm::my_function();
    println!("Handling request to {:?}", req.header("spin-full-url"));
    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body(format!("Hello, {answer:?}"))
        .build())
}
