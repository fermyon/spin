use anyhow::Result;
use http::{Method, StatusCode};
use spin_sdk::{
    http::{Request, Response},
    http_component,
    key_value::{Error, Store},
};

#[http_component]
fn handle_request(req: Request) -> Result<Response> {
    // Open the default key-value store
    let store = Store::open_default()?;

    let (status, body) = match *req.method() {
        Method::POST => {
            // Add the request (URI, body) tuple to the store
            store.set(req.uri().path(), req.body().as_deref().unwrap_or(&[]))?;
            (StatusCode::OK, None)
        }
        Method::GET => {
            // Get the value associated with the request URI, or return a 404 if it's not present
            match store.get(req.uri().path()) {
                Ok(value) => (StatusCode::OK, Some(value.into())),
                Err(Error::NoSuchKey) => (StatusCode::NOT_FOUND, None),
                Err(error) => return Err(error.into()),
            }
        }
        Method::DELETE => {
            // Delete the value associated with the request URI, if present
            store.delete(req.uri().path())?;
            (StatusCode::OK, None)
        }
        Method::HEAD => {
            // Like GET, except do not return the value
            match store.exists(req.uri().path()) {
                Ok(true) => (StatusCode::OK, None),
                Ok(false) => (StatusCode::NOT_FOUND, None),
                Err(error) => return Err(error.into()),
            }
        }
        // No other methods are currently supported
        _ => (StatusCode::METHOD_NOT_ALLOWED, None),
    };

    Ok(http::Response::builder().status(status).body(body)?)
}
