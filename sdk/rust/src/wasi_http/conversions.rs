use async_trait::async_trait;

use super::{Headers, IncomingRequest, Method, OutgoingResponse};

impl From<crate::http::Response> for OutgoingResponse {
    fn from(response: crate::http::Response) -> Self {
        let headers = response
            .headers
            .unwrap_or_default()
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
    R: crate::http::TryNonRequestFromRequest,
{
    type Error = IncomingRequestError<R::Error>;

    async fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        let req = crate::http::Request::try_from_incoming_request(request)
            .await
            .map_err(convert_error)?;
        R::try_from_request(req).map_err(IncomingRequestError::ConversionError)
    }
}

#[async_trait]
impl TryFromIncomingRequest for crate::http::Request {
    type Error = IncomingRequestError;

    async fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        let headers = request
            .headers()
            .entries()
            .iter()
            .map(|(k, v)| (k.clone(), String::from_utf8_lossy(v).into_owned()))
            .collect();
        Ok(Self {
            method: request
                .method()
                .try_into()
                .map_err(|_| IncomingRequestError::UnexpectedMethod(request.method()))?,
            uri: request
                .path_with_query()
                .unwrap_or_else(|| String::from("/")),
            headers,
            params: Vec::new(),
            body: Some(
                request
                    .into_body()
                    .await
                    .map_err(IncomingRequestError::BodyConversionError)?,
            ),
        })
    }
}

#[derive(Debug, thiserror::Error)]
/// An error converting an `IncomingRequest`
pub enum IncomingRequestError<E = std::convert::Infallible> {
    /// The `IncomingRequest` has a method not supported by `Request`
    #[error("unexpected method: {0:?}")]
    UnexpectedMethod(Method),
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
        IncomingRequestError::UnexpectedMethod(e) => IncomingRequestError::UnexpectedMethod(e),
        IncomingRequestError::BodyConversionError(e) => {
            IncomingRequestError::BodyConversionError(e)
        }
        IncomingRequestError::ConversionError(_) => unreachable!(),
    }
}

impl<E: crate::http::IntoResponse> crate::http::IntoResponse for IncomingRequestError<E> {
    fn into_response(self) -> crate::http::Response {
        match self {
            IncomingRequestError::UnexpectedMethod(_) => {
                crate::http::responses::method_not_allowed()
            }
            IncomingRequestError::BodyConversionError(e) => e.into_response(),
            IncomingRequestError::ConversionError(e) => e.into_response(),
        }
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
