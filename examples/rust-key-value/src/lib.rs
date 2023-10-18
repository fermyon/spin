use http::{Method, StatusCode};
use spin_sdk::{
    http::{IntoResponse, Response},
    http_component,
    key_value::Store,
};

#[http_component]
fn handle_request(req: http::Request<Vec<u8>>) -> anyhow::Result<impl IntoResponse> {
    // Open the default key-value store
    let store = Store::open_default()?;

    let (status, body) = match *req.method() {
        Method::POST => {
            // Add the request (URI, body) tuple to the store
            store.set(req.uri().path(), req.body().as_slice())?;
            (StatusCode::OK, None)
        }
        Method::GET => {
            // Get the value associated with the request URI, or return a 404 if it's not present
            match store.get(req.uri().path())? {
                Some(value) => (StatusCode::OK, Some(value)),
                None => (StatusCode::NOT_FOUND, None),
            }
        }
        Method::DELETE => {
            // Delete the value associated with the request URI, if present
            store.delete(req.uri().path())?;
            (StatusCode::OK, None)
        }
        Method::HEAD => {
            // Like GET, except do not return the value
            let code = if store.exists(req.uri().path())? {
                StatusCode::OK
            } else {
                StatusCode::NOT_FOUND
            };
            (code, None)
        }
        // No other methods are currently supported
        _ => (StatusCode::METHOD_NOT_ALLOWED, None),
    };
    Ok(Response::new(status, body))
}
