use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::wit::wasi::io::streams;

use super::{
    executor, Headers, IncomingRequest, IncomingResponse, Json, JsonBodyError, Lazy, LazyExt,
    OutgoingRequest, OutgoingResponse, RequestBuilder,
};

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
    fn try_from_incoming_request(value: IncomingRequest) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

#[async_trait]
impl TryFromIncomingRequest for IncomingRequest {
    type Error = std::convert::Infallible;

    fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        Ok(request)
    }
}

#[async_trait]
impl<R> TryFromIncomingRequest for R
where
    R: TryNonRequestFromRequest,
{
    type Error = IncomingRequestError<R::Error>;

    fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        let req = Request::try_from_incoming_request(request).unwrap();
        R::try_from_request(req).map_err(IncomingRequestError::ConversionError)
    }
}

#[async_trait]
impl TryFromIncomingRequest for Request {
    type Error = std::convert::Infallible;

    fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        Ok(Request::builder()
            .method(request.method())
            .uri(request.uri())
            .headers(request.headers())
            .body(
                <Lazy<Result<Vec<u8>, Arc<anyhow::Error>>> as LazyExt<_>>::new(move || {
                    executor::run(async move {
                        request
                            .into_body()
                            .await
                            .map_err(|e| Arc::new(anyhow::anyhow!("{}", e.to_debug_string())))
                    })
                }),
            )
            .build())
    }
}

#[derive(Debug, thiserror::Error)]
/// An error converting an `IncomingRequest`
pub enum IncomingRequestError<E = std::convert::Infallible> {
    /// There was an error converting the body to an `Option<Vec<u8>>`
    #[error(transparent)]
    BodyConversionError(anyhow::Error),
    /// There was an error converting the `Request` into the requested type
    #[error(transparent)]
    ConversionError(E),
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
impl<B: TryFromLazyBody> TryNonRequestFromRequest for hyperium::Request<B> {
    type Error = B::Error;

    fn try_from_request(req: Request) -> Result<Self, Self::Error> {
        let mut builder = hyperium::Request::builder()
            .uri(req.uri())
            .method(req.method);
        for (n, v) in req.headers {
            builder = builder.header(n, v.into_bytes());
        }
        Ok(builder.body(B::try_from_lazy_body(req.body)?).unwrap())
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
            super::Method::Connect => hyperium::Method::CONNECT,
            super::Method::Trace => hyperium::Method::TRACE,
            super::Method::Other(o) => hyperium::Method::from_bytes(o.as_bytes()).expect("TODO"),
        }
    }
}
impl From<hyperium::Method> for super::Method {
    fn from(method: hyperium::Method) -> Self {
        match method {
            hyperium::Method::GET => super::Method::Get,
            hyperium::Method::POST => super::Method::Post,
            hyperium::Method::PUT => super::Method::Put,
            hyperium::Method::DELETE => super::Method::Delete,
            hyperium::Method::PATCH => super::Method::Patch,
            hyperium::Method::HEAD => super::Method::Head,
            hyperium::Method::OPTIONS => super::Method::Options,
            hyperium::Method::CONNECT => super::Method::Connect,
            hyperium::Method::TRACE => super::Method::Trace,
            m => super::Method::Other(m.as_str().into()),
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
        responses::bad_request(Some(format!("invalid JSON body: {}", self.0)))
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

/// A trait for any type that can be turned into a `Request` body
pub trait IntoLazyBody {
    /// Turn `self` into a `Response` body
    fn into_lazy_body(self) -> Lazy<Result<Vec<u8>, Arc<anyhow::Error>>>;
}

impl<T: IntoLazyBody> IntoLazyBody for Option<T> {
    fn into_lazy_body(self) -> Lazy<Result<Vec<u8>, Arc<anyhow::Error>>> {
        self.map(|b| IntoLazyBody::into_lazy_body(b))
            .unwrap_or_else(|| LazyExt::new(|| Ok(Default::default())))
    }
}

impl IntoLazyBody for Lazy<Result<Vec<u8>, Arc<anyhow::Error>>> {
    fn into_lazy_body(self) -> Lazy<Result<Vec<u8>, Arc<anyhow::Error>>> {
        self
    }
}

impl IntoLazyBody for Vec<u8> {
    fn into_lazy_body(self) -> Lazy<Result<Vec<u8>, Arc<anyhow::Error>>> {
        LazyExt::new(move || Ok(self))
    }
}

#[cfg(feature = "http")]
impl IntoLazyBody for bytes::Bytes {
    fn into_lazy_body(self) -> Lazy<Result<Vec<u8>, Arc<anyhow::Error>>> {
        LazyExt::new(move || Ok(self.to_vec()))
    }
}

impl IntoLazyBody for () {
    fn into_lazy_body(self) -> Lazy<Result<Vec<u8>, Arc<anyhow::Error>>> {
        LazyExt::new(|| Ok(Default::default()))
    }
}

impl IntoLazyBody for &str {
    fn into_lazy_body(self) -> Lazy<Result<Vec<u8>, Arc<anyhow::Error>>> {
        let owned = self.to_owned();
        LazyExt::new(move || Ok(owned.into_bytes()))
    }
}

impl IntoLazyBody for String {
    fn into_lazy_body(self) -> Lazy<Result<Vec<u8>, Arc<anyhow::Error>>> {
        LazyExt::new(move || Ok(self.into_bytes()))
    }
}

/// A trait for any type that can be turned into a `Response` body
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

/// A trait for converting from a body or failing
pub trait TryFromLazyBody {
    /// The error encountered if conversion fails
    type Error: IntoResponse;
    /// Convert from a body to `Self` or fail
    fn try_from_lazy_body(
        body: Lazy<Result<Vec<u8>, Arc<anyhow::Error>>>,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

impl<T: TryFromLazyBody> TryFromLazyBody for Option<T> {
    type Error = T::Error;

    fn try_from_lazy_body(
        body: Lazy<Result<Vec<u8>, Arc<anyhow::Error>>>,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Some(TryFromLazyBody::try_from_lazy_body(body)?))
    }
}

impl TryFromLazyBody for String {
    type Error = anyhow::Error;

    fn try_from_lazy_body(
        body: Lazy<Result<Vec<u8>, Arc<anyhow::Error>>>,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        String::from_utf8(Lazy::force_value(body).map_err(|e| anyhow::anyhow!("{e}"))?)
            .map_err(|_| anyhow::Error::from(NonUtf8BodyError))
    }
}

impl TryFromLazyBody for Vec<u8> {
    type Error = anyhow::Error;

    fn try_from_lazy_body(
        body: Lazy<Result<Vec<u8>, Arc<anyhow::Error>>>,
    ) -> Result<Self, Self::Error> {
        Lazy::force_value(body).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

impl TryFromLazyBody for () {
    type Error = std::convert::Infallible;

    fn try_from_lazy_body(
        _body: Lazy<Result<Vec<u8>, Arc<anyhow::Error>>>,
    ) -> Result<Self, Self::Error> {
        Ok(())
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
impl<T: serde::de::DeserializeOwned> TryFromBody for Json<T> {
    type Error = JsonBodyError;
    fn try_from_body(body: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Json(serde_json::from_slice(&body).map_err(JsonBodyError)?))
    }
}

#[cfg(feature = "json")]
impl<T: serde::de::DeserializeOwned> TryFromLazyBody for Json<T> {
    type Error = anyhow::Error;

    fn try_from_lazy_body(
        body: Lazy<Result<Vec<u8>, Arc<anyhow::Error>>>,
    ) -> Result<Self, Self::Error> {
        Ok(Json(
            serde_json::from_slice(&Lazy::force_value(body).map_err(|e| anyhow::anyhow!("{e}"))?)
                .map_err(|e| anyhow::Error::from(JsonBodyError(e)))?,
        ))
    }
}

#[cfg(feature = "http")]
impl TryFromLazyBody for bytes::Bytes {
    type Error = anyhow::Error;

    fn try_from_lazy_body(
        body: Lazy<Result<Vec<u8>, Arc<anyhow::Error>>>,
    ) -> Result<Self, Self::Error> {
        Lazy::force_value(body)
            .map(Into::into)
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
}

#[cfg(feature = "json")]
impl<T: serde::Serialize + Send + Sync + 'static> IntoLazyBody for Json<T> {
    fn into_lazy_body(self) -> Lazy<Result<Vec<u8>, Arc<anyhow::Error>>> {
        LazyExt::new(move || {
            serde_json::to_vec(&self.0).map_err(|e| Arc::new(anyhow::Error::from(e)))
        })
    }
}

#[cfg(feature = "json")]
impl<T: serde::Serialize> TryIntoBody for Json<T> {
    type Error = JsonBodyError;

    fn try_into_body(self) -> Result<Vec<u8>, Self::Error> {
        serde_json::to_vec(&self.0).map_err(JsonBodyError)
    }
}

/// A trait for converting a type into an `OutgoingRequest`
pub trait TryIntoOutgoingRequest {
    /// The error if the conversion fails
    type Error;

    /// Turn the type into an `OutgoingRequest`
    ///
    /// If the implementor can be sure that the `OutgoingRequest::write` has not been called they
    /// can return a buffer as the second element of the returned tuple and `send` will send
    /// that as the request body.
    fn try_into_outgoing_request(self) -> Result<(OutgoingRequest, Option<Vec<u8>>), Self::Error>;
}

impl TryIntoOutgoingRequest for OutgoingRequest {
    type Error = std::convert::Infallible;

    fn try_into_outgoing_request(self) -> Result<(OutgoingRequest, Option<Vec<u8>>), Self::Error> {
        Ok((self, None))
    }
}

impl TryIntoOutgoingRequest for Request {
    type Error = anyhow::Error;

    fn try_into_outgoing_request(self) -> Result<(OutgoingRequest, Option<Vec<u8>>), Self::Error> {
        let headers = self
            .headers()
            .map(|(k, v)| (k.to_owned(), v.as_bytes().to_owned()))
            .collect::<Vec<_>>();
        let request = OutgoingRequest::new(
            self.method(),
            self.path_and_query(),
            Some(if self.is_https() {
                &super::Scheme::Https
            } else {
                &super::Scheme::Http
            }),
            self.authority(),
            &Headers::new(&headers),
        );
        Ok((
            request,
            Some(self.into_body().map_err(|e| anyhow::anyhow!("{e}"))?),
        ))
    }
}

impl TryIntoOutgoingRequest for RequestBuilder {
    type Error = anyhow::Error;

    fn try_into_outgoing_request(
        mut self,
    ) -> Result<(OutgoingRequest, Option<Vec<u8>>), Self::Error> {
        self.build().try_into_outgoing_request()
    }
}

#[cfg(feature = "http")]
impl<B> TryIntoOutgoingRequest for hyperium::Request<B>
where
    B: TryIntoBody,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = anyhow::Error;
    fn try_into_outgoing_request(self) -> Result<(OutgoingRequest, Option<Vec<u8>>), Self::Error> {
        let headers = self
            .headers()
            .into_iter()
            .map(|(n, v)| (n.as_str().to_owned(), v.as_bytes().to_owned()))
            .collect::<Vec<_>>();
        let request = OutgoingRequest::new(
            &self.method().clone().into(),
            self.uri().path_and_query().map(|p| p.as_str()),
            self.uri()
                .scheme()
                .map(|s| match s.as_str() {
                    "http" => super::Scheme::Http,
                    "https" => super::Scheme::Https,
                    s => super::Scheme::Other(s.to_owned()),
                })
                .as_ref(),
            self.uri().authority().map(|a| a.as_str()),
            &Headers::new(&headers),
        );
        let buffer = TryIntoBody::try_into_body(self.into_body())?;
        Ok((request, Some(buffer)))
    }
}

/// A trait for converting from an `IncomingRequest`
#[async_trait]
pub trait TryFromIncomingResponse {
    /// The error if conversion fails
    type Error;
    /// Turn the `IncomingResponse` into the type
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

#[async_trait]
impl TryFromIncomingResponse for Response {
    type Error = streams::Error;
    async fn try_from_incoming_response(resp: IncomingResponse) -> Result<Self, Self::Error> {
        Ok(Response::builder()
            .status(resp.status())
            .headers(resp.headers())
            .body(resp.into_body().await?)
            .build())
    }
}

#[cfg(feature = "http")]
#[async_trait]
impl<B: TryFromBody> TryFromIncomingResponse for hyperium::Response<B> {
    type Error = B::Error;
    async fn try_from_incoming_response(resp: IncomingResponse) -> Result<Self, Self::Error> {
        let mut builder = hyperium::Response::builder().status(resp.status());
        for (n, v) in resp.headers().entries() {
            builder = builder.header(n, v);
        }
        let body = resp.into_body().await.expect("TODO");
        Ok(builder.body(B::try_from_body(body)?).unwrap())
    }
}

/// Turn a type into a `Request`
pub trait IntoRequest {
    /// Turn `self` into a `Request`
    fn into_request(self) -> Request;
}

impl IntoRequest for Request {
    fn into_request(self) -> Request {
        self
    }
}

#[cfg(feature = "http")]
impl<B: IntoLazyBody> IntoRequest for hyperium::Request<B> {
    fn into_request(self) -> Request {
        Request::builder()
            .method(self.method().clone().into())
            .uri(self.uri().to_string())
            .headers(self.headers())
            .body(B::into_lazy_body(self.into_body()))
            .build()
    }
}
