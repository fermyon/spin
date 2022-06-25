//! The Rust Spin SDK.

#![deny(missing_docs)]

/// Exports the procedural macros for writing handlers for Spin components.
pub use spin_macro::*;

/// Exports the experimental outbound HTTP crate.
pub use wasi_experimental_http as outbound_http;

/// Helpers for building Spin HTTP components.
/// These are convenience helpers, and the types in this module are
/// based on the [`http`](https://crates.io/crates) crate.
pub mod http {
    use anyhow::Result;

    /// The Spin HTTP request.
    pub type Request = http::Request<Option<bytes::Bytes>>;

    /// The Spin HTTP response.
    pub type Response = http::Response<Option<bytes::Bytes>>;

    /// Directly expose the ability to send an HTTP request.
    pub use crate::outbound_http::send_request as send;

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

#[allow(missing_docs)]
pub mod redis {
    wit_bindgen_rust::import!("../../wit/ephemeral/outbound-redis.wit");

    /// Exports the generated outbound Redis items.
    pub use outbound_redis::*;
}

#[allow(missing_docs)]
pub mod pg {
    wit_bindgen_rust::import!("../../wit/ephemeral/outbound-pg.wit");

    /// Exports the generated outbound Pg items.
    pub use outbound_pg::*;
}
