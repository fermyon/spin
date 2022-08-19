use http::{header::HeaderName, HeaderValue, Uri};

use super::http::{Request, Response};

wit_bindgen_rust::import!("../../wit/ephemeral/wasi-outbound-http.wit");

use wasi_outbound_http::{
    HttpError as OutboundHttpError, Request as OutboundRequest, Response as OutboundResponse,
};

type Result<T> = std::result::Result<T, OutboundHttpError>;

/// Send an HTTP request and get a fully formed HTTP response.
pub fn send_request(req: Request) -> Result<Response> {
    let (req, body) = req.into_parts();

    let method = req.method.try_into()?;

    let (uri, params) = split_uri(req.uri)?;

    let params = &params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect::<Vec<_>>();

    let headers = &req
        .headers
        .iter()
        .map(try_header_to_strs)
        .collect::<Result<Vec<_>>>()?;

    let body = body.as_ref().map(|bytes| bytes.as_ref());

    let out_req = OutboundRequest {
        method,
        uri: &uri,
        params,
        headers,
        body,
    };

    let OutboundResponse {
        status,
        headers,
        body,
    } = wasi_outbound_http::request(out_req)?;

    let resp_builder = http::response::Builder::new().status(status);
    let resp_builder = headers
        .into_iter()
        .flatten()
        .fold(resp_builder, |b, (k, v)| b.header(k, v));
    resp_builder
        .body(body.map(Into::into))
        .map_err(|_| OutboundHttpError::RuntimeError)
}

fn split_uri(uri: Uri) -> Result<(String, Vec<(String, String)>)> {
    let params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes())
        .into_owned()
        .collect();
    let uri_without_params = {
        let mut parts = uri.into_parts();
        if let Some(paq) = parts.path_and_query.as_mut() {
            // Rebuild PathAndQuery from just path
            *paq = paq.path().try_into().unwrap();
        }
        Uri::from_parts(parts)
            .map_err(|_| OutboundHttpError::InvalidUrl)?
            .to_string()
    };
    Ok((uri_without_params, params))
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

impl TryFrom<http::Method> for wasi_outbound_http::Method {
    type Error = OutboundHttpError;

    fn try_from(method: http::Method) -> Result<Self> {
        use http::Method;
        use wasi_outbound_http::Method::*;
        Ok(match method {
            Method::GET => Get,
            Method::POST => Post,
            Method::PUT => Put,
            Method::DELETE => Delete,
            Method::PATCH => Patch,
            Method::HEAD => Head,
            Method::OPTIONS => Options,
            _ => return Err(wasi_outbound_http::HttpError::RequestError),
        })
    }
}

impl std::fmt::Display for OutboundHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for OutboundHttpError {}
