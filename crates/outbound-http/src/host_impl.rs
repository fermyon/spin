use anyhow::Result;
use http::{HeaderMap, Uri};
use reqwest::Client;
use spin_core::async_trait;
use spin_outbound_networking::{AllowedHostsConfig, OutboundUrl};
use spin_world::v1::{
    http as outbound_http,
    http_types::{self, Headers, HttpError, Method, Request, Response},
};
use tracing::{field::Empty, instrument, Level};

/// A very simple implementation for outbound HTTP requests.
#[derive(Default, Clone)]
pub struct OutboundHttp {
    /// List of hosts guest modules are allowed to make requests to.
    pub allowed_hosts: AllowedHostsConfig,
    /// During an incoming HTTP request, origin is set to the host of that incoming HTTP request.
    /// This is used to direct outbound requests to the same host when allowed.
    pub origin: String,
    client: Option<Client>,
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
    #[instrument(name = "spin_outbound_http.send_request", skip_all, err(level = Level::INFO),
        fields(otel.kind = "client", url.full = Empty, http.request.method = Empty,
        http.response.status_code = Empty, otel.name = Empty, server.address = Empty, server.port = Empty))]
    async fn send_request(&mut self, req: Request) -> Result<Response, HttpError> {
        let current_span = tracing::Span::current();
        let method = format!("{:?}", req.method)
            .strip_prefix("Method::")
            .unwrap_or("_OTHER")
            .to_uppercase();
        current_span.record("otel.name", method.clone());
        current_span.record("url.full", req.uri.clone());
        current_span.record("http.request.method", method);
        if let Ok(uri) = req.uri.parse::<Uri>() {
            if let Some(authority) = uri.authority() {
                current_span.record("server.address", authority.host());
                if let Some(port) = authority.port() {
                    current_span.record("server.port", port.as_u16());
                }
            }
        }

        tracing::trace!("Attempting to send outbound HTTP request to {}", req.uri);
        if !self
            .is_allowed(&req.uri)
            .map_err(|_| HttpError::RuntimeError)?
        {
            tracing::info!("Destination not allowed: {}", req.uri);
            if let Some((scheme, host_and_port)) = scheme_host_and_port(&req.uri) {
                terminal::warn!("A component tried to make a HTTP request to non-allowed host '{host_and_port}'.");
                eprintln!("To allow requests, add 'allowed_outbound_hosts = [\"{scheme}://{host_and_port}\"]' to the manifest component section.");
            }
            return Err(HttpError::DestinationNotAllowed);
        }

        let method = method_from(req.method);

        let abs_url = if req.uri.starts_with('/') {
            format!("{}{}", self.origin, req.uri)
        } else {
            req.uri.clone()
        };

        let req_url = reqwest::Url::parse(&abs_url).map_err(|_| HttpError::InvalidUrl)?;

        let mut headers = request_headers(req.headers).map_err(|_| HttpError::RuntimeError)?;
        spin_telemetry::inject_trace_context(&mut headers);
        let body = req.body.unwrap_or_default().to_vec();

        if !req.params.is_empty() {
            tracing::warn!("HTTP params field is deprecated");
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
        tracing::trace!("Returning response from outbound request to {}", req.uri);
        current_span.record("http.response.status_code", resp.status().as_u16());
        response_from_reqwest(resp).await
    }
}

impl http_types::Host for OutboundHttp {
    fn convert_http_error(&mut self, error: HttpError) -> Result<HttpError> {
        Ok(error)
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

fn method_from(m: Method) -> http::Method {
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

fn request_headers(h: Headers) -> anyhow::Result<HeaderMap> {
    let mut res = HeaderMap::new();
    for (k, v) in h {
        res.insert(
            http::header::HeaderName::try_from(k)?,
            http::header::HeaderValue::try_from(v)?,
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
