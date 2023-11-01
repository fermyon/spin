use anyhow::Result;
use spin_sdk::{
    http::{Request, Response},
    http_component, llm,
};

/// A simple Spin HTTP component.
#[http_component]
fn hello_world(_req: Request) -> Result<Response> {
    let model = llm::InferencingModel::Llama2Chat;
    let inference = llm::infer(model, "say hello")?;

    Ok(Response::builder().status(200).body(inference.text).build())
}
