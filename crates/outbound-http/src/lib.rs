mod host_component;

use futures::executor::block_on;
use http::HeaderMap;
use reqwest::{Client, Url};
use spin_manifest::AllowedHttpHosts;
use std::str::FromStr;
use wasi_outbound_http::*;
use wit_bindgen_wasmtime::async_trait;

pub use host_component::OutboundHttpComponent;
pub use wasi_outbound_http::add_to_linker;

wit_bindgen_wasmtime::export!({paths: ["../../wit/ephemeral/wasi-outbound-http.wit"], async: *});

pub const ALLOW_ALL_HOSTS: &str = "insecure:allow-all";

/// A very simple implementation for outbound HTTP requests.
#[derive(Default, Clone)]
pub struct OutboundHttp {
    /// List of hosts guest modules are allowed to make requests to.
    pub allowed_hosts: AllowedHttpHosts,
}

impl OutboundHttp {
    pub fn new(allowed_hosts: AllowedHttpHosts) -> Self {
        Self { allowed_hosts }
    }

    /// Check if guest module is allowed to send request to URL, based on the list of
    /// allowed hosts defined by the runtime. If the list of allowed hosts contains
    /// `insecure:allow-all`, then all hosts are allowed.
    /// If `None` is passed, the guest module is not allowed to send the request.
    fn is_allowed(&self, url: &str) -> Result<bool, HttpError> {
        let url = Url::parse(url).map_err(|_| HttpError::InvalidUrl)?;
        Ok(self.allowed_hosts.allow(&url))
    }
}

#[async_trait]
impl wasi_outbound_http::WasiOutboundHttp for OutboundHttp {
    async fn request(&mut self, req: Request<'_>) -> Result<Response, HttpError> {
        if !self.is_allowed(req.uri)? {
            tracing::log::info!("Destination not allowed: {}", req.uri);
            return Err(HttpError::DestinationNotAllowed);
        }

        let method = http::Method::from(req.method);
        let url = Url::parse(req.uri).map_err(|_| HttpError::InvalidUrl)?;
        let headers = request_headers(req.headers)?;
        let body = req.body.unwrap_or_default().to_vec();

        if !req.params.is_empty() {
            tracing::log::warn!("HTTP params field is deprecated");
        }

        let client = Client::builder().build().unwrap();
        let res = client
            .request(method, url)
            .headers(headers)
            .body(body)
            .send()
            .await;
        let resp = log_request_error(res)?;
        Response::try_from(resp).map_err(|_| HttpError::RuntimeError)
    }
}

fn log_request_error<R>(response: Result<R, reqwest::Error>) -> Result<R, reqwest::Error> {
    if let Err(e) = &response {
        let error_desc = if e.is_timeout() {
            "timeout error"
        } else if e.is_connect() {
            "connection error"
        } else if e.is_body() || e.is_decode() {
            "message body error"
        } else if e.is_request() {
            "request error"
        } else {
            "error"
        };
        tracing::warn!(
            "Outbound HTTP {}: URL {}, error detail {:?}",
            error_desc,
            e.url()
                .map(|u| u.to_string())
                .unwrap_or_else(|| "<unknown>".to_owned()),
            e
        );
    }
    response
}

impl From<Method> for http::Method {
    fn from(m: Method) -> Self {
        match m {
            Method::Get => http::Method::GET,
            Method::Post => http::Method::POST,
            Method::Put => http::Method::PUT,
            Method::Delete => http::Method::DELETE,
            Method::Patch => http::Method::PATCH,
            Method::Head => http::Method::HEAD,
            Method::Options => http::Method::OPTIONS,
        }
    }
}

impl TryFrom<reqwest::Response> for Response {
    type Error = HttpError;

    fn try_from(res: reqwest::Response) -> Result<Self, Self::Error> {
        let status = res.status().as_u16();
        let headers = response_headers(res.headers())?;

        let body = Some(block_on(res.bytes())?.to_vec());

        Ok(Response {
            status,
            headers,
            body,
        })
    }
}

impl TryFrom<reqwest::blocking::Response> for Response {
    type Error = HttpError;

    fn try_from(res: reqwest::blocking::Response) -> Result<Self, Self::Error> {
        let status = res.status().as_u16();
        let headers = response_headers(res.headers())?;
        let body = Some(res.bytes()?.to_vec());

        Ok(Response {
            status,
            headers,
            body,
        })
    }
}

fn request_headers(h: HeadersParam) -> anyhow::Result<HeaderMap> {
    let mut res = HeaderMap::new();
    for (k, v) in h {
        res.insert(
            http::header::HeaderName::from_str(k)?,
            http::header::HeaderValue::from_str(v)?,
        );
    }
    Ok(res)
}

fn response_headers(h: &HeaderMap) -> anyhow::Result<Option<Vec<(String, String)>>> {
    let mut res: Vec<(String, String)> = vec![];

    for (k, v) in h {
        res.push((
            k.to_string(),
            std::str::from_utf8(v.as_bytes())?.to_string(),
        ));
    }

    Ok(Some(res))
}

impl From<anyhow::Error> for HttpError {
    fn from(_: anyhow::Error) -> Self {
        Self::RuntimeError
    }
}

impl From<reqwest::Error> for HttpError {
    fn from(_: reqwest::Error) -> Self {
        Self::RequestError
    }
}
