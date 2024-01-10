use anyhow::Context as _;
use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::{http_component, key_value::Store};
use std::collections::HashMap;

/// A simple Spin HTTP component.
#[http_component]
fn hello_world(req: Request) -> anyhow::Result<impl IntoResponse> {
    let conn = Store::open_default()?;
    let query: HashMap<String, String> = serde_qs::from_str(req.query())?;
    let key = query.get("key").context("missing key query parameter")?;
    let value = conn.get(key)?.unwrap_or_else(|| "<none>".into());
    Ok(Response::new(200, value))
}
