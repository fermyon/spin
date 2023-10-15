use super::{Headers, IncomingRequest, Method, OutgoingResponse};

impl From<crate::http::Response> for OutgoingResponse {
    fn from(response: crate::http::Response) -> Self {
        // TODO: headers
        OutgoingResponse::new(response.status, &Headers::new(&[]))
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
        let req = crate::http::Request::try_from_incoming_request(request)?;
        R::try_from_request(req).map_err(|e| IncomingRequestError::ConversionError(e.into()))
    }
}

impl TryFromIncomingRequest for crate::http::Request {
    type Error = IncomingRequestError;

    fn try_from_incoming_request(request: IncomingRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            method: request
                .method()
                .try_into()
                .map_err(|_| IncomingRequestError::UnexpectedMethod(request.method()))?,
            uri: request
                .path_with_query()
                .unwrap_or_else(|| String::from("/")),
            headers: Vec::new(), // TODO
            params: Vec::new(),
            body: Some(
                request
                    .into_body_sync()
                    .map_err(IncomingRequestError::BodyConversionError)?,
            ),
        })
    }
}

#[derive(Debug, thiserror::Error)]
/// An error converting an `IncomingRequest`
pub enum IncomingRequestError {
    /// The `IncomingRequest` has a method not supported by `Request`
    #[error("unexpected method: {0:?}")]
    UnexpectedMethod(Method),
    /// There was an error converting the body to an `Option<Vec<u8>>k`
    #[error(transparent)]
    BodyConversionError(anyhow::Error),
    /// There was an error converting the `Request` into the requested type
    #[error(transparent)]
    ConversionError(Box<dyn std::error::Error>),
}

impl crate::http::IntoResponse for IncomingRequestError {
    fn into_response(self) -> crate::http::Response {
        match self {
            IncomingRequestError::UnexpectedMethod(_) => {
                crate::http::responses::method_not_allowed()
            }
            IncomingRequestError::BodyConversionError(e) => e.into_response(),
            IncomingRequestError::ConversionError(e) => crate::http::responses::bad_request(Some(
                format!("could not convert request to desired type: {e}"),
            )),
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
