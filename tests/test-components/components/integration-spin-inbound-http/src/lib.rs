wit_bindgen::generate!({
    world: "http-trigger",
    path: "../../../../wit/deps/spin@unversioned",
    exports: {
        "fermyon:spin/inbound-http": SpinHttp,
    }
});

use std::collections::HashMap;

use exports::fermyon::spin::inbound_http::{self, Request, Response};
use miniserde::json;

struct SpinHttp;

impl inbound_http::Guest for SpinHttp {
    fn handle_request(req: Request) -> Response {
        let (status, body) = match handle_request(req) {
            Ok(body) => (200, body),
            Err(err) => (500, format!("{err:?}").into_bytes()),
        };
        Response {
            status,
            headers: None,
            body: Some(body),
        }
    }
}

fn handle_request(req: Request) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if !req.params.is_empty() {
        return Err("request params field is deprecated".into());
    }

    let headers = req.headers.into_iter().collect::<HashMap<_, _>>();
    let path = headers
        .get("spin-path-info")
        .map(|path| path.as_str())
        .unwrap_or("");
    match path {
        "/echo" => Ok(req.body.unwrap_or_default()),
        "/assert-headers" => {
            let body = String::from_utf8(req.body.unwrap_or_default())?;
            let expected: HashMap<String, String> = json::from_str(&body)?;
            for (key, val) in expected {
                let got = headers
                    .get(&key)
                    .ok_or_else(|| format!("missing header {key:?}"))?;
                if got != &val {
                    return Err(format!("expected header {key}: {val:?}, got {got:?}").into());
                }
            }

            Ok(vec![])
        }
        other => Err(format!("unknown test route spin-path-info={other:?}").into()),
    }
}
