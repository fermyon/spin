use std::collections::HashMap;

use async_trait::async_trait;

use super::{Headers, IncomingRequest, IncomingResponse, OutgoingRequest, OutgoingResponse};

use super::{responses, NonUtf8BodyError, Request, Response};

impl From<Response> for OutgoingResponse {
    fn from(response: Response) -> Self {
        let headers = response
            .headers
            .into_iter()
            .map(|(k, v)| (k, v.into_bytes()))
            .collect::<Vec<_>>();
        OutgoingResponse::new(response.status, &Headers::new(&headers))
    }
}

/// A trait for trying to convert from an `IncomingRequest` to the implementing type
#[async_trait]
pub trait TryFromIncomingRequest {
    /// The error if conversion fails
    type Error;

    /// Try to turn the `IncomingRequest` into the implementing type
    async fn try_from_incoming_request(value: IncomingRequest) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

#[async_trait]
impl TryFromIncomingRequest for IncomingRequest {
    type Error = std::convert::Infallible;
    async fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        Ok(request)
    }
}

#[async_trait]
impl<R> TryFromIncomingRequest for R
where
    R: TryNonRequestFromRequest,
{
    type Error = IncomingRequestError<R::Error>;

    async fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        let req = Request::try_from_incoming_request(request)
            .await
            .map_err(convert_error)?;
        R::try_from_request(req).map_err(IncomingRequestError::ConversionError)
    }
}

#[async_trait]
impl TryFromIncomingRequest for Request {
    type Error = IncomingRequestError;

    async fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        Ok(Request::builder()
            .method(request.method())
            .uri(request.uri())
            .headers(request.headers())
            .body(
                request
                    .into_body()
                    .await
                    .map_err(IncomingRequestError::BodyConversionError)?,
            )
            .build())
    }
}

#[derive(Debug, thiserror::Error)]
/// An error converting an `IncomingRequest`
pub enum IncomingRequestError<E = std::convert::Infallible> {
    /// There was an error converting the body to an `Option<Vec<u8>>k`
    #[error(transparent)]
    BodyConversionError(anyhow::Error),
    /// There was an error converting the `Request` into the requested type
    #[error(transparent)]
    ConversionError(E),
}

/// Helper for converting `IncomingRequestError`s that cannot fail due to conversion errors
/// into ones that can.
fn convert_error<E>(
    error: IncomingRequestError<std::convert::Infallible>,
) -> IncomingRequestError<E> {
    match error {
        IncomingRequestError::BodyConversionError(e) => {
            IncomingRequestError::BodyConversionError(e)
        }
        IncomingRequestError::ConversionError(_) => unreachable!(),
    }
}

impl<E: IntoResponse> IntoResponse for IncomingRequestError<E> {
    fn into_response(self) -> Response {
        match self {
            IncomingRequestError::BodyConversionError(e) => e.into_response(),
            IncomingRequestError::ConversionError(e) => e.into_response(),
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
/// This is useful because we want to implement `TryFromIncomingRequest` for all types that impl
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
    type Error = HyperiumConversionError<B::Error>;
    fn try_from_request(req: Request) -> Result<Self, Self::Error> {
        let mut builder = hyperium::Request::builder()
            .uri(req.uri())
            .method(req.method);
        for (n, v) in req.headers {
            builder = builder.header(n, v.into_bytes());
        }
        builder
            .body(B::try_from_body(req.body).map_err(HyperiumConversionError::ConstructingBody)?)
            .map_err(HyperiumConversionError::Construction)
    }
}

#[cfg(feature = "http")]
impl TryFrom<super::Method> for hyperium::Method {
    type Error = hyperium::method::InvalidMethod;
    fn try_from(method: super::Method) -> Result<Self, Self::Error> {
        let method = match method {
            super::Method::Get => hyperium::Method::GET,
            super::Method::Post => hyperium::Method::POST,
            super::Method::Put => hyperium::Method::PUT,
            super::Method::Delete => hyperium::Method::DELETE,
            super::Method::Patch => hyperium::Method::PATCH,
            super::Method::Head => hyperium::Method::HEAD,
            super::Method::Options => hyperium::Method::OPTIONS,
            super::Method::Connect => hyperium::Method::CONNECT,
            super::Method::Trace => hyperium::Method::TRACE,
            super::Method::Other(o) => hyperium::Method::from_bytes(o.as_bytes())?,
        };
        Ok(method)
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
        Response::builder()
            .status(self.status().as_u16())
            .headers(self.headers())
            .body(self.into_body())
            .build()
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
            headers: Default::default(),
            body: body.as_bytes().to_vec(),
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
            headers: Default::default(),
            body: body.as_bytes().to_vec(),
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

/// A trait for any type that can be turned into `Response` headers
pub trait IntoHeaders {
    /// Turn `self` into `Response` headers
    fn into_headers(self) -> Vec<(String, Vec<u8>)>;
}

impl IntoHeaders for Vec<(String, String)> {
    fn into_headers(self) -> Vec<(String, Vec<u8>)> {
        self.into_iter().map(|(k, v)| (k, v.into_bytes())).collect()
    }
}

impl IntoHeaders for Vec<(String, Vec<u8>)> {
    fn into_headers(self) -> Vec<(String, Vec<u8>)> {
        self
    }
}

impl IntoHeaders for HashMap<String, Vec<String>> {
    fn into_headers(self) -> Vec<(String, Vec<u8>)> {
        self.into_iter()
            .flat_map(|(k, values)| values.into_iter().map(move |v| (k.clone(), v.into_bytes())))
            .collect()
    }
}

impl IntoHeaders for HashMap<String, String> {
    fn into_headers(self) -> Vec<(String, Vec<u8>)> {
        self.into_iter().map(|(k, v)| (k, v.into_bytes())).collect()
    }
}

impl IntoHeaders for HashMap<String, Vec<u8>> {
    fn into_headers(self) -> Vec<(String, Vec<u8>)> {
        self.into_iter().collect()
    }
}

impl IntoHeaders for &hyperium::HeaderMap {
    fn into_headers(self) -> Vec<(String, Vec<u8>)> {
        self.iter()
            .map(|(k, v)| (k.as_str().to_owned(), v.as_bytes().to_owned()))
            .collect()
    }
}

impl IntoHeaders for Headers {
    fn into_headers(self) -> Vec<(String, Vec<u8>)> {
        self.entries().into_headers()
    }
}

/// A trait for any type that can be turned into a `Response` body
pub trait IntoBody {
    /// Turn `self` into a `Response` body
    fn into_body(self) -> Vec<u8>;
}

impl<T: IntoBody> IntoBody for Option<T> {
    fn into_body(self) -> Vec<u8> {
        self.map(|b| IntoBody::into_body(b)).unwrap_or_default()
    }
}

impl IntoBody for Vec<u8> {
    fn into_body(self) -> Vec<u8> {
        self
    }
}

#[cfg(feature = "http")]
impl IntoBody for bytes::Bytes {
    fn into_body(self) -> Vec<u8> {
        self.to_vec()
    }
}

impl IntoBody for () {
    fn into_body(self) -> Vec<u8> {
        Default::default()
    }
}

impl IntoBody for &str {
    fn into_body(self) -> Vec<u8> {
        self.to_owned().into_bytes()
    }
}

impl IntoBody for String {
    fn into_body(self) -> Vec<u8> {
        self.to_owned().into_bytes()
    }
}

/// A trait for converting from a body or failing
pub trait TryFromBody {
    /// The error encountered if conversion fails
    type Error: IntoResponse;
    /// Convert from a body to `Self` or fail
    fn try_from_body(body: Vec<u8>) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

impl<T: TryFromBody> TryFromBody for Option<T> {
    type Error = T::Error;

    fn try_from_body(body: Vec<u8>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Some(TryFromBody::try_from_body(body)?))
    }
}

impl<T: FromBody> TryFromBody for T {
    type Error = std::convert::Infallible;

    fn try_from_body(body: Vec<u8>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(FromBody::from_body(body))
    }
}

impl TryFromBody for String {
    type Error = NonUtf8BodyError;

    fn try_from_body(body: Vec<u8>) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        String::from_utf8(body).map_err(|_| NonUtf8BodyError)
    }
}

#[cfg(feature = "json")]
impl<T: serde::de::DeserializeOwned> TryFromBody for super::Json<T> {
    type Error = super::JsonBodyError;
    fn try_from_body(body: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(super::Json(
            serde_json::from_slice(&body).map_err(super::JsonBodyError)?,
        ))
    }
}

/// A trait from converting from a body
pub trait FromBody {
    /// Convert from a body into the type
    fn from_body(body: Vec<u8>) -> Self;
}

impl FromBody for Vec<u8> {
    fn from_body(body: Vec<u8>) -> Self {
        body
    }
}

impl FromBody for () {
    fn from_body(_body: Vec<u8>) -> Self {}
}

#[cfg(feature = "http")]
impl FromBody for bytes::Bytes {
    fn from_body(body: Vec<u8>) -> Self {
        Into::into(body)
    }
}

/// A trait for any type that can be turned into a `Response` body or fail
pub trait TryIntoBody {
    /// The type of error if the conversion fails
    type Error;
    /// Turn `self` into an Error
    fn try_into_body(self) -> Result<Vec<u8>, Self::Error>;
}

impl<B> TryIntoBody for B
where
    B: IntoBody,
{
    type Error = std::convert::Infallible;

    fn try_into_body(self) -> Result<Vec<u8>, Self::Error> {
        Ok(self.into_body())
    }
}

impl<T> TryIntoBody for super::Json<T>
where
    T: serde::Serialize,
{
    type Error = serde_json::Error;
    fn try_into_body(self) -> Result<Vec<u8>, Self::Error> {
        serde_json::to_vec(&self.0)
    }
}

#[cfg(feature = "http")]
impl<B> TryFrom<hyperium::Request<B>> for OutgoingRequest
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
            .map(|(n, v)| (n.as_str().to_owned(), v.as_bytes().to_owned()))
            .collect::<Vec<_>>();
        Ok(OutgoingRequest::new(
            &method,
            req.uri().path_and_query().map(|p| p.as_str()),
            req.uri()
                .scheme()
                .map(|s| match s.as_str() {
                    "http" => super::Scheme::Http,
                    "https" => super::Scheme::Https,
                    s => super::Scheme::Other(s.to_owned()),
                })
                .as_ref(),
            req.uri().authority().map(|a| a.as_str()),
            &Headers::new(&headers),
        ))
    }
}

#[async_trait]
/// Convert to a type from an `IncomingResponse`
pub trait TryFromIncomingResponse {
    /// The error from converting from an `IncomingResponse`
    type Error;
    /// Convert to the type from an `IncomingResponse`
    async fn try_from_incoming_response(resp: IncomingResponse) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

#[async_trait]
impl TryFromIncomingResponse for IncomingResponse {
    type Error = std::convert::Infallible;
    async fn try_from_incoming_response(resp: IncomingResponse) -> Result<Self, Self::Error> {
        Ok(resp)
    }
}

#[cfg(feature = "http")]
#[async_trait]
impl<B: TryFromBody> TryFromIncomingResponse for hyperium::Response<B> {
    type Error = HyperiumConversionError<B::Error>;
    async fn try_from_incoming_response(resp: IncomingResponse) -> Result<Self, Self::Error> {
        let mut builder = hyperium::Response::builder().status(resp.status());
        for (n, v) in resp.headers().entries() {
            builder = builder.header(n, v);
        }
        let body = resp
            .into_body()
            .await
            .map_err(HyperiumConversionError::CollectingBody)?;
        builder
            .body(B::try_from_body(body).map_err(HyperiumConversionError::ConstructingBody)?)
            .map_err(HyperiumConversionError::Construction)
    }
}

#[cfg(feature = "http")]
#[derive(Debug)]
/// An error constructing a hyperium `http` crate type
pub enum HyperiumConversionError<B> {
    /// Error constructing the `http` crate type
    Construction(hyperium::Error),
    /// Error collecting the body bytes from the body stream
    CollectingBody(anyhow::Error),
    /// Error creating the body type
    ConstructingBody(B),
}

#[cfg(feature = "http")]
impl<B: IntoResponse> IntoResponse for HyperiumConversionError<B> {
    fn into_response(self) -> Response {
        match self {
            HyperiumConversionError::Construction(e) => {
                // TODO: This should probably be a 400 not a 500
                (Box::new(e) as Box<dyn std::error::Error>).into_response()
            }
            HyperiumConversionError::CollectingBody(e) => e.into_response(),
            HyperiumConversionError::ConstructingBody(b) => b.into_response(),
        }
    }
}

#[cfg(feature = "http")]
impl<B: std::error::Error> std::fmt::Display for HyperiumConversionError<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HyperiumConversionError::Construction(e) => write!(f, "{e}"),
            HyperiumConversionError::CollectingBody(e) => write!(f, "{e}"),
            HyperiumConversionError::ConstructingBody(e) => write!(f, "{e}"),
        }
    }
}

impl<B: std::error::Error> std::error::Error for HyperiumConversionError<B> {}
