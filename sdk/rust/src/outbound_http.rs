use super::http::{Request, Response};
use http_types::{header::HeaderName, HeaderValue};

use super::wit::http_types::{
    HttpError as OutboundHttpError, Method, Request as Req, Response as Resp,
};
use super::wit::outbound_http;

type Result<T> = std::result::Result<T, OutboundHttpError>;

/// Send an HTTP request and get a fully formed HTTP response.
pub fn send_request(req: Request) -> Result<Response> {
    let (req, body) = req.into_parts();

    let method = req.method.try_into()?;

    let uri = req.uri.to_string();

    let params = vec![];

    let headers = &req
        .headers
        .iter()
        .map(try_header_to_strs)
        .collect::<Result<Vec<_>>>()?;

    let body = body.map(|bytes| bytes.to_vec());

    let out_req = Req {
        method,
        uri: &uri,
        params: &params,
        headers,
        body: body.as_deref(),
    };

    let Resp {
        status,
        headers,
        body,
    } = outbound_http::request(out_req)?;

    let resp_builder = http_types::response::Builder::new().status(status);
    let resp_builder = headers
        .into_iter()
        .flatten()
        .fold(resp_builder, |b, (k, v)| b.header(k, v));
    resp_builder
        .body(body.map(Into::into))
        .map_err(|_| OutboundHttpError::RuntimeError)
}

fn try_header_to_strs<'k, 'v>(
    header: (&'k HeaderName, &'v HeaderValue),
) -> Result<(&'k str, &'v str)> {
    Ok((
        header.0.as_str(),
        header
            .1
            .to_str()
            .map_err(|_| OutboundHttpError::InvalidUrl)?,
    ))
}

impl TryFrom<http_types::Method> for Method {
    type Error = OutboundHttpError;

    fn try_from(method: http_types::Method) -> Result<Self> {
        use http_types::Method;
        Ok(match method {
            Method::GET => Self::Get,
            Method::POST => Self::Post,
            Method::PUT => Self::Put,
            Method::DELETE => Self::Delete,
            Method::PATCH => Self::Patch,
            Method::HEAD => Self::Head,
            Method::OPTIONS => Self::Options,
            _ => return Err(OutboundHttpError::RequestError),
        })
    }
}

impl std::fmt::Display for OutboundHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for OutboundHttpError {}
