use crate::wit::v1::http::{Request, Response};

use super::{responses, NonUtf8BodyError};

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

#[cfg(feature = "http")]
impl<B: TryFromBody> TryNonRequestFromRequest for hyperium::Request<B> {
    type Error = B::Error;
    fn try_from_request(req: Request) -> Result<Self, Self::Error> {
        let mut builder = hyperium::Request::builder().uri(req.uri).method(req.method);
        for (n, v) in req.headers {
            builder = builder.header(n, v);
        }
        Ok(builder.body(B::try_from_body(req.body)?).unwrap())
    }
}

#[cfg(feature = "http")]
impl From<super::Method> for hyperium::Method {
    fn from(method: super::Method) -> Self {
        match method {
            super::Method::Get => hyperium::Method::GET,
            super::Method::Post => hyperium::Method::POST,
            super::Method::Put => hyperium::Method::PUT,
            super::Method::Delete => hyperium::Method::DELETE,
            super::Method::Patch => hyperium::Method::PATCH,
            super::Method::Head => hyperium::Method::HEAD,
            super::Method::Options => hyperium::Method::OPTIONS,
        }
    }
}

/// A trait for any type that can be turned into a `Response`
pub trait IntoResponse {
    /// Turn `self` into a `Response`
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

#[cfg(feature = "http")]
impl<B> IntoResponse for hyperium::Response<B>
where
    B: IntoBody,
{
    fn into_response(self) -> Response {
        let headers = self
            .headers()
            .into_iter()
            .map(|(n, v)| {
                (
                    n.as_str().to_owned(),
                    String::from_utf8_lossy(v.as_bytes()).into_owned(),
                )
            })
            .collect();
        Response::new_with_headers(
            self.status().as_u16(),
            headers,
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
impl IntoResponse for super::JsonBodyError {
    fn into_response(self) -> Response {
        responses::bad_request(Some(format!("failed to parse JSON body: {}", self.0)))
    }
}

impl IntoResponse for NonUtf8BodyError {
    fn into_response(self) -> Response {
        responses::bad_request(Some(
            "expected body to be a utf8 string but wasn't".to_owned(),
        ))
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
impl IntoStatusCode for hyperium::StatusCode {
    fn into_status_code(self) -> u16 {
        self.as_u16()
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

#[cfg(feature = "http")]
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
impl<T: serde::de::DeserializeOwned> TryFromBody for super::Json<T> {
    type Error = super::JsonBodyError;
    fn try_from_body(body: Option<Vec<u8>>) -> Result<Self, Self::Error> {
        Ok(super::Json(
            serde_json::from_slice(&body.unwrap_or_default()).map_err(super::JsonBodyError)?,
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
    type Error = std::convert::Infallible;

    fn try_into_body(self) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.into_body())
    }
}

#[cfg(feature = "http")]
impl<B> TryFrom<hyperium::Request<B>> for Request
where
    B: TryIntoBody,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = anyhow::Error;
    fn try_from(req: hyperium::Request<B>) -> Result<Self, Self::Error> {
        let method = match req.method() {
            &hyperium::Method::GET => super::Method::Get,
            &hyperium::Method::POST => super::Method::Post,
            &hyperium::Method::PUT => super::Method::Put,
            &hyperium::Method::DELETE => super::Method::Delete,
            &hyperium::Method::PATCH => super::Method::Patch,
            &hyperium::Method::HEAD => super::Method::Head,
            &hyperium::Method::OPTIONS => super::Method::Options,
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
            body: B::try_into_body(req.into_body())?,
        })
    }
}

#[cfg(feature = "http")]
impl<B: TryFromBody> TryFrom<Response> for hyperium::Response<B> {
    type Error = B::Error;
    fn try_from(resp: Response) -> Result<Self, Self::Error> {
        let mut builder = hyperium::Response::builder().status(resp.status);
        for (n, v) in resp.headers.unwrap_or_default() {
            builder = builder.header(n, v);
        }
        Ok(builder.body(B::try_from_body(resp.body)?).unwrap())
    }
}
