//! The Rust Spin SDK.

#![deny(missing_docs)]

/// Outbound HTTP request functionality.
pub mod outbound_http;

/// Key/Value storage.
pub mod key_value;

/// Exports the procedural macros for writing handlers for Spin components.
pub use spin_macro::*;

/// Helpers for building Spin HTTP components.
/// These are convenience helpers, and the types in this module are
/// based on the [`http`](https://crates.io/crates) crate.
pub mod http {
    use anyhow::Result;

    /// The Spin HTTP request.
    pub type Request = http::Request<Option<bytes::Bytes>>;

    /// The Spin HTTP response.
    pub type Response = http::Response<Option<bytes::Bytes>>;

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

/// Implementation of the spin redis interface.
#[allow(missing_docs)]
pub mod redis {
    use std::hash::{Hash, Hasher};

    wit_bindgen_rust::import!("../../wit/ephemeral/outbound-redis.wit");

    impl PartialEq for outbound_redis::ValueResult {
        fn eq(&self, other: &Self) -> bool {
            use outbound_redis::ValueResult::*;

            match (self, other) {
                (Nil, Nil) => true,
                (String(a), String(b)) => a == b,
                (Int(a), Int(b)) => a == b,
                (Data(a), Data(b)) => a == b,
                _ => false,
            }
        }
    }

    impl Eq for outbound_redis::ValueResult {}

    impl Hash for outbound_redis::ValueResult {
        fn hash<H: Hasher>(&self, state: &mut H) {
            use outbound_redis::ValueResult::*;

            match self {
                Nil => (),
                String(s) => s.hash(state),
                Int(v) => v.hash(state),
                Data(v) => v.hash(state),
            }
        }
    }

    /// Exports the generated outbound Redis items.
    pub use outbound_redis::*;
}

/// Implementation of the spin postgres db interface.
pub mod pg;

/// Implementation of the Spin MySQL database interface.
pub mod mysql;

/// Implementation of the spin config interface.
#[allow(missing_docs)]
pub mod config {
    wit_bindgen_rust::import!("../../wit/ephemeral/spin-config.wit");

    /// Exports the generated Spin config items.
    pub use spin_config::{get_config as get, Error};

    impl ::std::fmt::Display for Error {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            match self {
                Error::Provider(provider_err) => write!(f, "provider error: {}", provider_err),
                Error::InvalidKey(invalid_key) => write!(f, "invalid key: {}", invalid_key),
                Error::InvalidSchema(invalid_schema) => {
                    write!(f, "invalid schema: {}", invalid_schema)
                }
                Error::Other(other) => write!(f, "other: {}", other),
            }
        }
    }

    impl ::std::error::Error for Error {}
}
