//! The Rust Spin SDK.

#![deny(missing_docs)]

/// Outbound HTTP request functionality.
pub mod outbound_http;

/// Key/Value storage.
pub mod key_value;

/// SQLite storage.
pub mod sqlite;

/// Exports the procedural macros for writing handlers for Spin components.
pub use spin_macro::*;

#[doc(hidden)]
/// Module containing wit bindgen generated code.
///
/// This is only meant for internal consumption.
pub mod wit {
    #![allow(missing_docs)]
    wit_bindgen::generate!({
        world: "reactor",
        path: "../../wit/preview2",
        macro_call_prefix: "::spin_sdk::wit::",
        duplicate_if_necessary,
        macro_export
    });
}

/// Needed by the export macro
///
/// See [this commit](https://github.com/bytecodealliance/wit-bindgen/pull/394/commits/9d2ea88f986f4a883ba243449e3a070cac18958e) for more info.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use wit::__link_section;

#[export_name = concat!("spin-sdk-version-", env!("SDK_VERSION"))]
extern "C" fn __spin_sdk_version() {}

#[cfg(feature = "export-sdk-language")]
#[export_name = "spin-sdk-language-rust"]
extern "C" fn __spin_sdk_language() {}

#[export_name = concat!("spin-sdk-commit-", env!("SDK_COMMIT"))]
extern "C" fn __spin_sdk_hash() {}

/// Helpers for building Spin HTTP components.
/// These are convenience helpers, and the types in this module are
/// based on the [`http`](https://crates.io/crates) crate.
pub mod http {
    use anyhow::Result;

    /// The Spin HTTP request.
    pub type Request = http_types::Request<Option<bytes::Bytes>>;

    /// The Spin HTTP response.
    pub type Response = http_types::Response<Option<bytes::Bytes>>;

    pub use crate::outbound_http::send_request as send;

    /// Exports HTTP Router items.
    pub use router::*;
    mod router;

    /// Helper function to return a 404 Not Found response.
    pub fn not_found() -> Result<Response> {
        Ok(http_types::Response::builder()
            .status(404)
            .body(Some("Not Found".into()))?)
    }

    /// Helper function to return a 500 Internal Server Error response.
    pub fn internal_server_error() -> Result<Response> {
        Ok(http_types::Response::builder()
            .status(500)
            .body(Some("Internal Server Error".into()))?)
    }
}

/// Implementation of the spin redis interface.
#[allow(missing_docs)]
pub mod redis {
    use std::hash::{Hash, Hasher};

    pub use super::wit::fermyon::spin::redis::{
        del, execute, get, incr, publish, sadd, set, smembers, srem,
    };
    pub use super::wit::fermyon::spin::redis_types::*;

    impl PartialEq for RedisResult {
        fn eq(&self, other: &Self) -> bool {
            use super::wit::fermyon::spin::redis_types::RedisResult::*;
            match (self, other) {
                (Nil, Nil) => true,
                (Status(a), Status(b)) => a == b,
                (Int64(a), Int64(b)) => a == b,
                (Binary(a), Binary(b)) => a == b,
                _ => false,
            }
        }
    }

    impl Eq for RedisResult {}

    impl Hash for RedisResult {
        fn hash<H: Hasher>(&self, state: &mut H) {
            use super::wit::fermyon::spin::redis_types::RedisResult::*;

            match self {
                Nil => (),
                Status(s) => s.hash(state),
                Int64(v) => v.hash(state),
                Binary(v) => v.hash(state),
            }
        }
    }
}

/// Implementation of the spin postgres db interface.
pub mod pg;

/// Implementation of the Spin MySQL database interface.
pub mod mysql;

/// Implementation of the spin config interface.
#[allow(missing_docs)]
pub mod config {
    /// Exports the generated Spin config items.
    pub use super::wit::fermyon::spin::config::{get_config as get, Error};
}

/// Inbound http trigger functionality
// Hide the docs since this is only needed for the macro
#[doc(hidden)]
pub mod inbound_http {
    use super::wit::exports::fermyon::spin::inbound_http;
    use super::wit::fermyon::spin::http_types as spin_http_types;
    pub use inbound_http::*;

    impl TryFrom<inbound_http::Request> for http_types::Request<Option<bytes::Bytes>> {
        type Error = anyhow::Error;

        fn try_from(spin_req: inbound_http::Request) -> Result<Self, Self::Error> {
            let mut http_req = http_types::Request::builder()
                .method(spin_req.method)
                .uri(&spin_req.uri);

            append_request_headers(&mut http_req, &spin_req)?;

            let body = match spin_req.body {
                Some(b) => b.to_vec(),
                None => Vec::new(),
            };

            let body = Some(bytes::Bytes::from(body));

            Ok(http_req.body(body)?)
        }
    }

    impl From<spin_http_types::Method> for http_types::Method {
        fn from(spin_method: spin_http_types::Method) -> Self {
            match spin_method {
                spin_http_types::Method::Get => http_types::Method::GET,
                spin_http_types::Method::Post => http_types::Method::POST,
                spin_http_types::Method::Put => http_types::Method::PUT,
                spin_http_types::Method::Delete => http_types::Method::DELETE,
                spin_http_types::Method::Patch => http_types::Method::PATCH,
                spin_http_types::Method::Head => http_types::Method::HEAD,
                spin_http_types::Method::Options => http_types::Method::OPTIONS,
            }
        }
    }

    fn append_request_headers(
        http_req: &mut http_types::request::Builder,
        spin_req: &inbound_http::Request,
    ) -> anyhow::Result<()> {
        let headers = http_req.headers_mut().unwrap();
        for (k, v) in &spin_req.headers {
            headers.append(
                <http_types::header::HeaderName as std::str::FromStr>::from_str(k)?,
                http_types::header::HeaderValue::from_str(v)?,
            );
        }

        Ok(())
    }

    impl TryFrom<inbound_http::Response> for http_types::Response<Option<bytes::Bytes>> {
        type Error = anyhow::Error;

        fn try_from(spin_res: inbound_http::Response) -> Result<Self, Self::Error> {
            let mut http_res = http_types::Response::builder().status(spin_res.status);
            append_response_headers(&mut http_res, spin_res.clone())?;

            let body = match spin_res.body {
                Some(b) => b.to_vec(),
                None => Vec::new(),
            };
            let body = Some(bytes::Bytes::from(body));

            Ok(http_res.body(body)?)
        }
    }

    fn append_response_headers(
        http_res: &mut http_types::response::Builder,
        spin_res: inbound_http::Response,
    ) -> anyhow::Result<()> {
        let headers = http_res.headers_mut().unwrap();
        for (k, v) in spin_res.headers.unwrap() {
            headers.append(
                <http_types::header::HeaderName as ::std::str::FromStr>::from_str(&k)?,
                http_types::header::HeaderValue::from_str(&v)?,
            );
        }

        Ok(())
    }

    impl TryFrom<http_types::Response<Option<bytes::Bytes>>> for inbound_http::Response {
        type Error = anyhow::Error;

        fn try_from(
            http_res: http_types::Response<Option<bytes::Bytes>>,
        ) -> Result<Self, Self::Error> {
            let status = http_res.status().as_u16();
            let headers = Some(outbound_headers(http_res.headers())?);
            let body = http_res.body().as_ref().map(|b| b.to_vec());

            Ok(inbound_http::Response {
                status,
                headers,
                body,
            })
        }
    }

    fn outbound_headers(hm: &http_types::HeaderMap) -> anyhow::Result<Vec<(String, String)>> {
        let mut res = Vec::new();

        for (k, v) in hm {
            res.push((
                k.as_str().to_string(),
                std::str::from_utf8(v.as_bytes())?.to_string(),
            ));
        }

        Ok(res)
    }
}

/// Inbound redis trigger functionality
// Hide the docs since this is only needed for the macro
#[doc(hidden)]
pub mod inbound_redis {
    pub use super::wit::exports::fermyon::spin::inbound_redis::*;
}
