use futures::{Sink, Stream};

#[doc(inline)]
pub use super::wit::wasi::http::types::*;

impl IncomingRequest {
    /// Return a `Stream` from which the body of the specified request may be read.
    pub fn into_body_stream(self) -> impl Stream<Item = anyhow::Result<Vec<u8>>> {
        executor::incoming_body(self.consume().expect("request should be consumable"))
    }

    /// Return a `Vec<u8>` of the body
    pub async fn into_body(self) -> anyhow::Result<Vec<u8>> {
        use futures::TryStreamExt;
        let mut stream = self.into_body_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.try_next().await? {
            body.extend(chunk);
        }
        Ok(body)
    }
    /// Return a `Vec<u8>` of the body
    pub fn into_body_sync(self) -> anyhow::Result<Vec<u8>> {
        let future = async { self.into_body().await };
        futures::pin_mut!(future);
        executor::run(future)
    }
}

impl IncomingResponse {
    /// Return a `Stream` from which the body of the specified response may be read.
    pub fn into_body(self) -> impl Stream<Item = anyhow::Result<Vec<u8>>> {
        executor::incoming_body(self.consume().expect("response should be consumable"))
    }
}

impl OutgoingResponse {
    /// Construct a `Sink` which writes chunks to the body of the specified response.
    pub fn take_body(&self) -> impl Sink<Vec<u8>, Error = anyhow::Error> {
        executor::outgoing_body(self.write().expect("response should be writable"))
    }
}

impl ResponseOutparam {
    /// Set with the outgoing response and the supplied buffer
    ///
    /// Will panic if response body has already been taken
    pub async fn set_with_body(
        self,
        response: OutgoingResponse,
        buffer: Vec<u8>,
    ) -> anyhow::Result<()> {
        use futures::SinkExt;
        let mut body = response.take_body();
        ResponseOutparam::set(self, Ok(response));
        body.send(buffer).await
    }
}

/// Send an outgoing request
pub async fn send(request: OutgoingRequest) -> Result<IncomingResponse, Error> {
    executor::outgoing_request_send(request).await
}

#[doc(hidden)]
/// The executor for driving wasi-http futures to completion
pub mod executor;

#[doc(inline)]
pub use executor::run;

impl TryFrom<IncomingRequest> for crate::http::Request {
    type Error = anyhow::Error;
    fn try_from(req: IncomingRequest) -> Result<Self, Self::Error> {
        let method = req
            .method()
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid method"))?;
        let uri = req.path_with_query().unwrap(); // TODO
        let future = async move { req.into_body().await };
        futures::pin_mut!(future);
        let body = executor::run(future)?;
        Ok(Self {
            method,
            uri,
            headers: Vec::new(), //TODO
            params: Vec::new(),
            body: Some(body),
        })
    }
}

impl TryFrom<Method> for crate::http::Method {
    type Error = ();
    fn try_from(method: Method) -> Result<Self, Self::Error> {
        let method = match method {
            Method::Get => Self::Get,
            Method::Head => Self::Head,
            Method::Post => Self::Post,
            Method::Put => Self::Put,
            Method::Patch => Self::Patch,
            Method::Delete => Self::Delete,
            Method::Options => Self::Options,
            _ => return Err(()),
        };
        Ok(method)
    }
}

impl From<crate::http::Response> for (OutgoingResponse, Vec<u8>) {
    fn from(response: crate::http::Response) -> Self {
        // TODO: headers
        (
            OutgoingResponse::new(response.status, &Headers::new(&[])),
            response.body.unwrap_or_default(),
        )
    }
}

/// A trait for trying to convert from an `IncomingRequest` to the implementing type
pub trait TryFromIncomingRequest {
    /// The error if conversion fails
    type Error;

    /// Try to turn the `IncomingRequest` into the implementing type
    fn try_from_incoming_request(value: IncomingRequest) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

#[derive(Debug, thiserror::Error)]
/// TODO
pub enum IncomingRequestError {
    /// TODO
    #[error(transparent)]
    ToRequest(anyhow::Error),
    /// TODO
    #[error(transparent)]
    ToType(Box<dyn std::error::Error>),
}

impl crate::http::IntoResponse for IncomingRequestError {
    fn into_response(self) -> crate::http::Response {
        match self {
            IncomingRequestError::ToRequest(e) => e.into_response(),
            IncomingRequestError::ToType(e) => e.into_response(),
        }
    }
}
impl TryFromIncomingRequest for IncomingRequest {
    type Error = std::convert::Infallible;
    fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        Ok(request)
    }
}

impl<R> TryFromIncomingRequest for R
where
    R: crate::http::TryFromRequest,
    R::Error: Into<Box<dyn std::error::Error>>,
{
    type Error = IncomingRequestError;

    fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        let req = crate::http::Request::try_from_incoming_request(request)
            .map_err(IncomingRequestError::ToRequest)?;
        R::try_from_request(req).map_err(|e| IncomingRequestError::ToType(e.into()))
    }
}

impl TryFromIncomingRequest for crate::http::Request {
    type Error = anyhow::Error;

    fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            method: request
                .method()
                .try_into()
                .map_err(|_| anyhow::anyhow!("unexpected method"))?,
            uri: request
                .path_with_query()
                .unwrap_or_else(|| String::from("/")),
            headers: Vec::new(), // TODO
            params: Vec::new(),
            body: Some(request.into_body_sync()?),
        })
    }
}
