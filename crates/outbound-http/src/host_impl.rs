use anyhow::Result;
use reqwest::Client;
use spin_core::async_trait;
use spin_outbound_networking::{AllowedHostsConfig, OutboundUrl};
use spin_world::v1::{
    http as outbound_http,
    http_types::{Headers, HttpError, Method, Request, Response},
};

/// A very simple implementation for outbound HTTP requests.
#[derive(Default, Clone)]
pub struct OutboundHttp {
    /// List of hosts guest modules are allowed to make requests to.
    pub allowed_hosts: AllowedHostsConfig,
    /// Used to dispatch outbound `self` requests directly to a component.
    pub self_dispatcher: HttpSelfDispatcher,
    client: Option<Client>,
}

#[derive(Default, Clone)]
pub enum HttpSelfDispatcher {
    #[default]
    NotHttp,
    Handler(std::sync::Arc<Box<dyn HttpRequestHandler + Send + Sync>>),
}

#[async_trait]
pub trait HttpRequestHandler {
    async fn handle(
        &self,
        mut req: http::Request<wasmtime_wasi_http::body::HyperIncomingBody>,
        scheme: http::uri::Scheme,
        addr: std::net::SocketAddr,
    ) -> anyhow::Result<http::Response<wasmtime_wasi_http::body::HyperIncomingBody>>;
}

impl HttpSelfDispatcher {
    pub fn new(handler: &std::sync::Arc<Box<dyn HttpRequestHandler + Send + Sync>>) -> Self {
        Self::Handler(handler.clone())
    }

    async fn dispatch(&self, request: Request) -> Result<Response, HttpError> {
        match self {
            Self::NotHttp => {
                tracing::error!("Cannot send request to {}: same-application requests are supported only for applications with HTTP triggers", request.uri);
                Err(HttpError::RuntimeError)
            }
            Self::Handler(handler) => {
                let mut reqbuilder = http::Request::builder()
                    .uri(request.uri)
                    .method(http_method_from(request.method));
                for (hname, hval) in request.headers {
                    reqbuilder = reqbuilder.header(hname, hval);
                }
                let req = reqbuilder
                    .body(match request.body {
                        Some(b) => spin_http::body::full(b.into()),
                        None => spin_http::body::empty(),
                    })
                    .map_err(|_| HttpError::RuntimeError)?;
                let scheme = http::uri::Scheme::HTTPS;
                let addr = std::net::SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                    0,
                );
                let resp = handler
                    .handle(req, scheme, addr)
                    .await
                    .map_err(|_| HttpError::RuntimeError)?;
                Ok(Response {
                    status: resp.status().as_u16(),
                    headers: None,
                    body: None,
                })
            }
        }
    }
}

impl OutboundHttp {
    /// Check if guest module is allowed to send request to URL, based on the list of
    /// allowed hosts defined by the runtime. If the url passed in is a relative path,
    /// only allow if allowed_hosts contains `self`. If the list of allowed hosts contains
    /// `insecure:allow-all`, then all hosts are allowed.
    /// If `None` is passed, the guest module is not allowed to send the request.
    fn is_allowed(&mut self, url: &str) -> Result<bool, HttpError> {
        if url.starts_with('/') {
            return Ok(self.allowed_hosts.allows_relative_url(&["http", "https"]));
        }

        Ok(OutboundUrl::parse(url, "https")
            .map(|u| self.allowed_hosts.allows(&u))
            .unwrap_or_default())
    }
}

#[async_trait]
impl outbound_http::Host for OutboundHttp {
    async fn send_request(&mut self, req: Request) -> Result<Result<Response, HttpError>> {
        Ok(async {
            tracing::log::trace!("Attempting to send outbound HTTP request to {}", req.uri);
            if !self
                .is_allowed(&req.uri)
                .map_err(|_| HttpError::RuntimeError)?
            {
                tracing::log::info!("Destination not allowed: {}", req.uri);
                if let Some((scheme, host_and_port)) = scheme_host_and_port(&req.uri) {
                    terminal::warn!("A component tried to make a HTTP request to non-allowed host '{host_and_port}'.");
                    eprintln!("To allow requests, add 'allowed_outbound_hosts = [\"{scheme}://{host_and_port}\"]' to the manifest component section.");
                }
                return Err(HttpError::DestinationNotAllowed);
            }

            if req.uri.starts_with('/') {
                return self.self_dispatcher.dispatch(req).await;
            }

            let method = reqwest_method_from(req.method);

            let abs_url = req.uri.clone();

            let req_url = reqwest::Url::parse(&abs_url).map_err(|_| HttpError::InvalidUrl)?;

            let headers = request_headers(req.headers).map_err(|_| HttpError::RuntimeError)?;
            let body = req.body.unwrap_or_default().to_vec();

            if !req.params.is_empty() {
                tracing::log::warn!("HTTP params field is deprecated");
            }

            // Allow reuse of Client's internal connection pool for multiple requests
            // in a single component execution
            let client = self.client.get_or_insert_with(Default::default);

            let resp = client
                .request(method, req_url)
                .headers(headers)
                .body(body)
                .send()
                .await
                .map_err(log_reqwest_error)?;
            tracing::log::trace!("Returning response from outbound request to {}", req.uri);
            response_from_reqwest(resp).await
        }
        .await)
    }
}

fn log_reqwest_error(err: reqwest::Error) -> HttpError {
    let error_desc = if err.is_timeout() {
        "timeout error"
    } else if err.is_connect() {
        "connection error"
    } else if err.is_body() || err.is_decode() {
        "message body error"
    } else if err.is_request() {
        "request error"
    } else {
        "error"
    };
    tracing::warn!(
        "Outbound HTTP {}: URL {}, error detail {:?}",
        error_desc,
        err.url()
            .map(|u| u.to_string())
            .unwrap_or_else(|| "<unknown>".to_owned()),
        err
    );
    HttpError::RuntimeError
}

fn http_method_from(m: Method) -> http::Method {
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

fn reqwest_method_from(m: Method) -> reqwest::Method {
    match m {
        Method::Get => reqwest::Method::GET,
        Method::Post => reqwest::Method::POST,
        Method::Put => reqwest::Method::PUT,
        Method::Delete => reqwest::Method::DELETE,
        Method::Patch => reqwest::Method::PATCH,
        Method::Head => reqwest::Method::HEAD,
        Method::Options => reqwest::Method::OPTIONS,
    }
}

async fn response_from_reqwest(res: reqwest::Response) -> Result<Response, HttpError> {
    let status = res.status().as_u16();
    let headers = response_headers(res.headers()).map_err(|_| HttpError::RuntimeError)?;

    let body = Some(
        res.bytes()
            .await
            .map_err(|_| HttpError::RuntimeError)?
            .to_vec(),
    );

    Ok(Response {
        status,
        headers,
        body,
    })
}

fn request_headers(h: Headers) -> anyhow::Result<reqwest::header::HeaderMap> {
    let mut res = reqwest::header::HeaderMap::new();
    for (k, v) in h {
        res.insert(
            reqwest::header::HeaderName::try_from(k)?,
            reqwest::header::HeaderValue::try_from(v)?,
        );
    }
    Ok(res)
}

fn response_headers(
    h: &reqwest::header::HeaderMap,
) -> anyhow::Result<Option<Vec<(String, String)>>> {
    let mut res: Vec<(String, String)> = vec![];

    for (k, v) in h {
        res.push((
            k.to_string(),
            std::str::from_utf8(v.as_bytes())?.to_string(),
        ));
    }

    Ok(Some(res))
}

/// Returns both the scheme and the `$HOST:$PORT` for the url string
///
/// Returns `None` if the url cannot be parsed or if it does not contain a host
fn scheme_host_and_port(url: &str) -> Option<(String, String)> {
    url::Url::parse(url).ok().and_then(|u| {
        u.host_str().map(|h| {
            let mut host = h.to_owned();
            if let Some(p) = u.port() {
                use std::fmt::Write;
                write!(&mut host, ":{p}").unwrap();
            }
            (u.scheme().to_owned(), host)
        })
    })
}
