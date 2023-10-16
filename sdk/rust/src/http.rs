use std::hash::Hash;
use std::{convert::Infallible, fmt::Display};

use crate::wit::v1::{http::send_request, http_types::HttpError};

#[doc(inline)]
pub use crate::wit::v1::http_types::{Method, Request, Response};

/// Perform an HTTP request getting back a response or an error
pub fn send<I, O>(req: I) -> Result<O, SendError>
where
    I: TryInto<Request>,
    I::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    O: TryFrom<Response>,
    O::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
{
    let response = send_request(
        &req.try_into()
            .map_err(|e| SendError::RequestConversion(e.into()))?,
    )
    .map_err(SendError::Http)?;
    response
        .try_into()
        .map_err(|e: O::Error| SendError::ResponseConversion(e.into()))
}

/// An error encountered when performing an HTTP request
#[derive(thiserror::Error, Debug)]
pub enum SendError {
    /// Error converting to a request
    #[error(transparent)]
    RequestConversion(Box<dyn std::error::Error + Send + Sync>),
    /// Error converting from a response
    #[error(transparent)]
    ResponseConversion(Box<dyn std::error::Error + Send + Sync>),
    /// An HTTP error
    #[error(transparent)]
    Http(HttpError),
}

#[cfg(feature = "http")]
impl<B> TryFrom<http_types::Request<B>> for Request
where
    B: TryIntoBody,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = anyhow::Error;
    fn try_from(req: http_types::Request<B>) -> Result<Self, Self::Error> {
        let method = match req.method() {
            &http_types::Method::GET => Method::Get,
            &http_types::Method::POST => Method::Post,
            &http_types::Method::PUT => Method::Put,
            &http_types::Method::DELETE => Method::Delete,
            &http_types::Method::PATCH => Method::Patch,
            &http_types::Method::HEAD => Method::Head,
            &http_types::Method::OPTIONS => Method::Options,
            m => anyhow::bail!("Unsupported method: {m}"),
        };
        let headers = req
            .headers()
            .into_iter()
            .map(|(n, v)| {
                (
                    n.as_str().to_owned(),
                    String::from_utf8_lossy(v.as_bytes()).into_owned(),
                )
            })
            .collect();
        Ok(Request {
            method,
            uri: req.uri().to_string(),
            headers,
            params: Vec::new(),
            body: TryIntoBody::try_into_body(req.into_body())?,
        })
    }
}

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

/// A trait for any type that can be constructor from a `Request`
pub trait TryFromRequest {
    /// The error if the conversion fails
    type Error;
    /// Try to turn the request into the type
    fn try_from_request(req: Request) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

impl TryFromRequest for Request {
    type Error = std::convert::Infallible;

    fn try_from_request(req: Request) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(req)
    }
}

impl<R: TryNonRequestFromRequest> TryFromRequest for R {
    type Error = R::Error;

    fn try_from_request(req: Request) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        TryNonRequestFromRequest::try_from_request(req)
    }
}

/// A hack that allows us to do blanket impls for `T where T: TryFromRequest` for all types
/// `T` *except* for `Request`.
///
/// This is useful in `wasi_http` where we want to implement `TryFromIncomingRequest` for all types that impl
/// `TryFromRequest` with the exception of `Request` itself. This allows that implementation to first convert
/// the `IncomingRequest` to a `Request` and then using this trait convert from `Request` to the given type.
pub trait TryNonRequestFromRequest {
    /// The error if the conversion fails
    type Error;
    /// Try to turn the request into the type
    fn try_from_request(req: Request) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

impl<B: TryFromBody> TryNonRequestFromRequest for Body<B> {
    type Error = B::Error;
    fn try_from_request(req: Request) -> Result<Self, Self::Error> {
        Ok(Body(B::try_from_body(req.body)?))
    }
}

#[cfg(feature = "json")]
impl<B: serde::de::DeserializeOwned> TryNonRequestFromRequest for Json<B> {
    type Error = JsonBodyError;
    fn try_from_request(req: Request) -> Result<Self, Self::Error> {
        Ok(Json(
            serde_json::from_slice(&req.body.unwrap_or_default()).map_err(JsonBodyError)?,
        ))
    }
}

/// An error parsing a JSON body
#[cfg(feature = "json")]
#[derive(Debug)]
pub struct JsonBodyError(serde_json::Error);

impl std::error::Error for JsonBodyError {}

impl Display for JsonBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("could not convert body to json")
    }
}

/// An error when the body is not UTF-8
#[derive(Debug)]
pub struct NonUtf8BodyError;

impl std::error::Error for NonUtf8BodyError {}

impl Display for NonUtf8BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("body was expected to be utf8 but was not")
    }
}

#[cfg(feature = "http")]
impl<B: TryFromBody> TryNonRequestFromRequest for http_types::Request<B> {
    type Error = B::Error;
    fn try_from_request(req: Request) -> Result<Self, Self::Error> {
        let mut builder = http_types::Request::builder()
            .uri(req.uri)
            .method(req.method);
        for (n, v) in req.headers {
            builder = builder.header(n, v);
        }
        Ok(builder.body(B::try_from_body(req.body)?).unwrap())
    }
}

#[cfg(feature = "http")]
impl From<Method> for http_types::Method {
    fn from(method: Method) -> Self {
        match method {
            Method::Get => http_types::Method::GET,
            Method::Post => http_types::Method::POST,
            Method::Put => http_types::Method::PUT,
            Method::Delete => http_types::Method::DELETE,
            Method::Patch => http_types::Method::PATCH,
            Method::Head => http_types::Method::HEAD,
            Method::Options => http_types::Method::OPTIONS,
        }
    }
}

#[cfg(feature = "http")]
impl<B: TryFromBody> TryFrom<Response> for http_types::Response<B> {
    type Error = B::Error;
    fn try_from(resp: Response) -> Result<Self, Self::Error> {
        let mut builder = http_types::Response::builder().status(resp.status);
        for (n, v) in resp.headers.unwrap_or_default() {
            builder = builder.header(n, v);
        }
        Ok(builder.body(B::try_from_body(resp.body)?).unwrap())
    }
}

mod router;
/// Exports HTTP Router items.
pub use router::*;

/// A trait for any type that can be turned into a `Response`
pub trait IntoResponse {
    /// Turn `self` into a `Response`
    fn into_response(self) -> Response;
}

impl<R: Into<Response>> IntoResponse for R {
    fn into_response(self) -> Response {
        self.into()
    }
}

impl<S: IntoStatusCode, B: IntoBody> IntoResponse for (S, B) {
    fn into_response(self) -> Response {
        Response {
            status: self.0.into_status_code(),
            headers: Default::default(),
            body: self.1.into_body(),
        }
    }
}

#[cfg(feature = "http")]
impl<B> IntoResponse for http_types::Response<B>
where
    B: IntoBody,
{
    fn into_response(self) -> Response {
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
            IntoBody::into_body(self.into_body()),
        )
    }
}

impl<R: IntoResponse, E: IntoResponse> IntoResponse for std::result::Result<R, E> {
    fn into_response(self) -> Response {
        match self {
            Ok(r) => r.into_response(),
            Err(e) => e.into_response(),
        }
    }
}

impl IntoResponse for anyhow::Error {
    fn into_response(self) -> Response {
        let body = self.to_string();
        eprintln!("Handler returned an error: {}", body);
        let mut source = self.source();
        while let Some(s) = source {
            eprintln!("  caused by: {}", s);
            source = s.source();
        }
        Response {
            status: 500,
            headers: None,
            body: Some(body.as_bytes().to_vec()),
        }
    }
}

impl IntoResponse for Box<dyn std::error::Error> {
    fn into_response(self) -> Response {
        let body = self.to_string();
        eprintln!("Handler returned an error: {}", body);
        let mut source = self.source();
        while let Some(s) = source {
            eprintln!("  caused by: {}", s);
            source = s.source();
        }
        Response {
            status: 500,
            headers: None,
            body: Some(body.as_bytes().to_vec()),
        }
    }
}

#[cfg(feature = "json")]
impl IntoResponse for JsonBodyError {
    fn into_response(self) -> Response {
        responses::bad_request(Some(format!("failed to parse JSON body: {}", self.0)))
    }
}

impl IntoResponse for NonUtf8BodyError {
    fn into_response(self) -> Response {
        responses::bad_request(Some("expected body to be utf8 but wasn't".to_owned()))
    }
}

impl IntoResponse for std::convert::Infallible {
    fn into_response(self) -> Response {
        unreachable!()
    }
}

/// A trait for any type that can be turned into a `Response` status code
pub trait IntoStatusCode {
    /// Turn `self` into a status code
    fn into_status_code(self) -> u16;
}

impl IntoStatusCode for u16 {
    fn into_status_code(self) -> u16 {
        self
    }
}

#[cfg(feature = "http")]
impl IntoStatusCode for http_types::StatusCode {
    fn into_status_code(self) -> u16 {
        self.as_u16()
    }
}

/// A trait for any type that can be turned into a `Response` body or fail
pub trait TryIntoBody {
    /// The type of error if the conversion fails
    type Error;
    /// Turn `self` into an Error
    fn try_into_body(self) -> Result<Option<Vec<u8>>, Self::Error>;
}

impl<B> TryIntoBody for B
where
    B: IntoBody,
{
    type Error = Infallible;

    fn try_into_body(self) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.into_body())
    }
}

/// A trait for any type that can be turned into a `Response` body
pub trait IntoBody {
    /// Turn `self` into a `Response` body
    fn into_body(self) -> Option<Vec<u8>>;
}

impl<T: IntoBody> IntoBody for Option<T> {
    fn into_body(self) -> Option<Vec<u8>> {
        self.and_then(|b| IntoBody::into_body(b))
    }
}

impl IntoBody for Vec<u8> {
    fn into_body(self) -> Option<Vec<u8>> {
        Some(self)
    }
}

impl IntoBody for bytes::Bytes {
    fn into_body(self) -> Option<Vec<u8>> {
        Some(self.to_vec())
    }
}

impl IntoBody for () {
    fn into_body(self) -> Option<Vec<u8>> {
        None
    }
}

impl IntoBody for &str {
    fn into_body(self) -> Option<Vec<u8>> {
        Some(self.to_owned().into_bytes())
    }
}

impl IntoBody for String {
    fn into_body(self) -> Option<Vec<u8>> {
        Some(self.to_owned().into_bytes())
    }
}

/// A trait for converting from a body or failing
pub trait TryFromBody {
    /// The error encountered if conversion fails
    type Error: IntoResponse;
    /// Convert from a body to `Self` or fail
    fn try_from_body(body: Option<Vec<u8>>) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

impl<T: TryFromBody> TryFromBody for Option<T> {
    type Error = T::Error;

    fn try_from_body(body: Option<Vec<u8>>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(match body {
            None => None,
            Some(v) => Some(TryFromBody::try_from_body(Some(v))?),
        })
    }
}

impl<T: FromBody> TryFromBody for T {
    type Error = std::convert::Infallible;

    fn try_from_body(body: Option<Vec<u8>>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(FromBody::from_body(body))
    }
}

impl TryFromBody for String {
    type Error = NonUtf8BodyError;

    fn try_from_body(body: Option<Vec<u8>>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        String::from_utf8(body.unwrap_or_default()).map_err(|_| NonUtf8BodyError)
    }
}

#[cfg(feature = "json")]
impl<T: serde::de::DeserializeOwned> TryFromBody for Json<T> {
    type Error = JsonBodyError;
    fn try_from_body(body: Option<Vec<u8>>) -> Result<Self, Self::Error> {
        Ok(Json(
            serde_json::from_slice(&body.unwrap_or_default()).map_err(JsonBodyError)?,
        ))
    }
}

/// A trait from converting from a body
pub trait FromBody {
    /// Convert from a body into the type
    fn from_body(body: Option<Vec<u8>>) -> Self;
}

impl FromBody for Vec<u8> {
    fn from_body(body: Option<Vec<u8>>) -> Self {
        body.unwrap_or_default()
    }
}

impl FromBody for () {
    fn from_body(_body: Option<Vec<u8>>) -> Self {}
}

#[cfg(feature = "http")]
impl FromBody for bytes::Bytes {
    fn from_body(body: Option<Vec<u8>>) -> Self {
        Into::into(body.unwrap_or_default())
    }
}

/// A Body extractor
#[derive(Debug)]
pub struct Body<T>(pub T);

impl<T> std::ops::Deref for Body<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A Json extractor
#[derive(Debug)]
pub struct Json<T>(pub T);

impl<T> std::ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
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

    pub(crate) fn bad_request(msg: Option<String>) -> Response {
        Response::new(400, msg.map(|m| m.into_bytes()))
    }
}
