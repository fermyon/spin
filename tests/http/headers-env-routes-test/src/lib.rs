use anyhow::Result;
use spin_sdk::{http::{Request, Response}, http_component};
use std::collections::HashMap;
use std::convert::TryInto;

#[http_component]
fn hello_world(mut req: Request) -> Result<Response> {
    let headers = env_to_headers();
    let req_headers = req.headers_mut();
    // NOTE: The change between the old code's append_request_headers function and that provided by the Rust SDK don't line up as far as I can tell and that lead me
    //       into a minor fight with the compiler. The code below feels like a mess and should be looked at with some scrutiny.
    let mut name_map = HashMap::new();
    let _ = headers
        .into_iter()
        .map(|(k,v)| name_map.insert(k, v));
    let _ = req_headers
        .iter()
        .map(|(k, v)| name_map.insert(String::from(k.as_str()), String::from(v.to_str().unwrap())));
    
    let res_headers_map : http::HeaderMap = (&name_map).try_into()?;
    
    let mut res = http::Response::builder()
        .status(200);
    
    let res_headers = res.headers_mut().unwrap();

    let _ = res_headers_map
        .iter()
        .map(|(k,v)| res_headers.insert(k, http::HeaderValue::clone(v)));

    Ok(res.body(Some("I'm a teapot".into()))?)
}

fn env_to_headers() -> Vec<(String, String)> {
    let mut res = vec![];
    std::env::vars().for_each(|(k, v)| res.push((format!("ENV_{}", k), v)));

    res
}
