//! The Rust Spin SDK.

#![deny(missing_docs)]

/// Outbound HTTP request functionality.
pub mod outbound_http;

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
    wit_bindgen_rust::import!("../../wit/ephemeral/outbound-redis.wit");

    /// Exports the generated outbound Redis items.
    pub use outbound_redis::*;
}

/// Implementation of the spin postgres db interface.
#[allow(missing_docs)]
pub mod pg {
    wit_bindgen_rust::import!("../../wit/ephemeral/outbound-pg.wit");

    /// Exports the generated outbound Pg items.
    pub use outbound_pg::*;

    impl TryFrom<&DbValue> for i32 {
        type Error = anyhow::Error;

        fn try_from(value: &DbValue) -> Result<Self, Self::Error> {
            match value {
                DbValue::Int32(n) => Ok(*n),
                _ => Err(anyhow::anyhow!(
                    "Expected integer from database but got {:?}",
                    value
                )),
            }
        }
    }

    impl TryFrom<&DbValue> for String {
        type Error = anyhow::Error;

        fn try_from(value: &DbValue) -> Result<Self, Self::Error> {
            match value {
                DbValue::Str(s) => Ok(s.to_owned()),
                _ => Err(anyhow::anyhow!(
                    "Expected string from the DB but got {:?}",
                    value
                )),
            }
        }
    }

    impl TryFrom<&DbValue> for i64 {
        type Error = anyhow::Error;

        fn try_from(value: &DbValue) -> Result<Self, Self::Error> {
            match value {
                DbValue::Int64(n) => Ok(*n),
                _ => Err(anyhow::anyhow!(
                    "Expected integer from the DB but got {:?}",
                    value
                )),
            }
        }
    }
}

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
