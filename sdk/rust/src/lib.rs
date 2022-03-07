//! The Rust Spin SDK.

#![deny(missing_docs)]

/// Export the macros
pub use spin_macro::*;

/// Export the experimental outbound HTTP crate.
pub use wasi_experimental_http as outbound_http;

/// HTTP helpers.
pub mod http {
    use anyhow::Result;

    /// The Spin HTTP request.
    pub type Request = http::Request<Option<bytes::Bytes>>;

    /// The Spin HTTP response.
    pub type Response = http::Response<Option<bytes::Bytes>>;

    /// Helper function to return a 404 Not Found response.
    pub fn not_found() -> Result<Response> {
        Ok(http::Response::builder()
            .status(404)
            .body(Some("Not Found".into()))?)
    }

    /// Helper function to return a 500 Internal Server Error response.
    pub fn internal_server_error() -> Result<Response> {
        Ok(http::Response::builder()
            .status(500)
            .body(Some("Internal Server Error".into()))?)
    }
}
