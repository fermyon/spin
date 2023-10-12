use std::fmt::Display;
use std::hash::Hash;

pub use crate::wit::v1::http::send_request as send;

pub use crate::wit::v1::http_types::{Method, Request, Response};

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
        })
    }
}

impl Hash for Method {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl Response {
    /// Create a new response from a status and optional headers and body
    pub fn new(status: u16, body: Option<Vec<u8>>) -> Self {
        Self {
            status,
            headers: None,
            body,
        }
    }

    /// Create a new response from a status and optional headers and body
    pub fn new_with_headers(
        status: u16,
        headers: Vec<(String, String)>,
        body: Option<Vec<u8>>,
    ) -> Self {
        Self {
            status,
            headers: Some(headers),
            body,
        }
    }
}

#[cfg(feature = "http")]
impl<B: FromBody> TryFrom<Request> for http_types::Request<Option<B>> {
    type Error = ();
    fn try_from(value: Request) -> Result<Self, Self::Error> {
        let method = match value.method {
            Method::Get => http_types::Method::GET,
            Method::Post => http_types::Method::POST,
            Method::Put => http_types::Method::PUT,
            Method::Delete => http_types::Method::DELETE,
            Method::Patch => http_types::Method::PATCH,
            Method::Head => http_types::Method::HEAD,
            Method::Options => http_types::Method::OPTIONS,
        };
        let mut builder = http_types::Request::builder().uri(value.uri).method(method);
        for (n, v) in value.headers {
            builder = builder.header(n, v);
        }
        Ok(builder.body(value.body.map(B::from)).unwrap())
    }
}

mod router;
/// Exports HTTP Router items.
pub use router::*;

/// A trait for any type that can be turned into a `Response`
pub trait IntoResponse {
    /// Turn `self` into a `Response`
    fn into(self) -> Response;
}

impl<S: IntoStatusCode, B: IntoBody> IntoResponse for (S, B) {
    fn into(self) -> Response {
        Response {
            status: self.0.into(),
            headers: Default::default(),
            body: self.1.into(),
        }
    }
}

#[cfg(feature = "http")]
impl<B> IntoResponse for http_types::Response<B>
where
    for<'a> &'a B: IntoBody,
{
    fn into(self) -> Response {
        Response::new_with_headers(
            self.status().as_u16(),
            self.headers()
                .into_iter()
                .map(|(n, v)| {
                    (
                        n.as_str().to_owned(),
                        String::from_utf8_lossy(v.as_bytes()).into_owned(),
                    )
                })
                .collect(),
            IntoBody::into(self.body()),
        )
    }
}

/// A trait for any type that can be turned into a `Response` status code
pub trait IntoStatusCode {
    /// Turn `self` into a status code
    fn into(self) -> u16;
}

impl IntoStatusCode for u16 {
    fn into(self) -> u16 {
        self
    }
}

#[cfg(feature = "http")]
impl IntoStatusCode for http_types::StatusCode {
    fn into(self) -> u16 {
        self.as_u16()
    }
}

/// A trait for any type that can be turned into a `Response` body
pub trait IntoBody {
    /// Turn `self` into a `Response` body
    fn into(self) -> Option<Vec<u8>>;
}

impl IntoBody for Option<bytes::Bytes> {
    fn into(self) -> Option<Vec<u8>> {
        self.map(|b| b.to_vec())
    }
}

impl IntoBody for &Option<bytes::Bytes> {
    fn into(self) -> Option<Vec<u8>> {
        self.as_ref().map(|b| b.to_vec())
    }
}

impl IntoBody for &str {
    fn into(self) -> Option<Vec<u8>> {
        Some(self.to_owned().into_bytes())
    }
}

/// A trait from converting from a body
pub trait FromBody {
    /// Convert from a body into the type
    fn from(body: Vec<u8>) -> Self;
}

impl FromBody for Vec<u8> {
    fn from(body: Vec<u8>) -> Self {
        body
    }
}

impl FromBody for String {
    fn from(body: Vec<u8>) -> Self {
        String::from_utf8_lossy(&body).into_owned()
    }
}

#[cfg(feature = "http")]
impl FromBody for bytes::Bytes {
    fn from(body: Vec<u8>) -> Self {
        body.into()
    }
}

/// Helper functions for creating responses
pub mod responses {
    use super::Response;

    /// Helper function to return a 404 Not Found response.
    pub fn not_found() -> Response {
        Response::new(404, Some("Not Found".into()))
    }

    /// Helper function to return a 500 Internal Server Error response.
    pub fn internal_server_error() -> Response {
        Response::new(500, Some("Internal Server Error".into()))
    }

    /// Helper function to return a 405 Method Not Allowed response.
    pub fn method_not_allowed() -> Response {
        Response::new(405, Some("Method Not Allowed".into()))
    }
}
