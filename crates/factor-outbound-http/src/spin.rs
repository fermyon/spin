use spin_world::{
    async_trait,
    v1::{
        http as spin_http,
        http_types::{self, HttpError, Method, Request, Response},
    },
};
use tracing::{field::Empty, instrument, Level, Span};

#[async_trait]
impl spin_http::Host for crate::InstanceState {
    #[instrument(name = "spin_outbound_http.send_request", skip_all, err(level = Level::INFO),
        fields(otel.kind = "client", url.full = Empty, http.request.method = Empty,
        http.response.status_code = Empty, otel.name = Empty, server.address = Empty, server.port = Empty))]
    async fn send_request(&mut self, req: Request) -> Result<Response, HttpError> {
        let span = Span::current();
        record_request_fields(&span, &req);

        let uri = req.uri;
        tracing::trace!("Sending outbound HTTP to {uri:?}");

        let abs_url = if !uri.starts_with('/') {
            // Absolute URI
            let is_allowed = self
                .allowed_hosts
                .check_url(&uri, "https")
                .await
                .unwrap_or(false);
            if !is_allowed {
                return Err(HttpError::DestinationNotAllowed);
            }
            uri
        } else {
            // Relative URI ("self" request)
            let is_allowed = self
                .allowed_hosts
                .check_relative_url(&["http", "https"])
                .await
                .unwrap_or(false);
            if !is_allowed {
                return Err(HttpError::DestinationNotAllowed);
            }

            let Some(origin) = &self.self_request_origin else {
                tracing::error!(
                    "Couldn't handle outbound HTTP request to relative URI; no origin set"
                );
                return Err(HttpError::InvalidUrl);
            };
            format!("{origin}{uri}")
        };
        let req_url = reqwest::Url::parse(&abs_url).map_err(|_| HttpError::InvalidUrl)?;

        if !req.params.is_empty() {
            tracing::warn!("HTTP params field is deprecated");
        }

        // Allow reuse of Client's internal connection pool for multiple requests
        // in a single component execution
        let client = self.spin_http_client.get_or_insert_with(Default::default);

        let mut req = {
            let mut builder = client.request(reqwest_method(req.method), req_url);
            for (key, val) in req.headers {
                builder = builder.header(key, val);
            }
            builder
                .body(req.body.unwrap_or_default())
                .build()
                .map_err(|err| {
                    tracing::error!("Error building outbound request: {err}");
                    HttpError::RuntimeError
                })?
        };
        spin_telemetry::inject_trace_context(req.headers_mut());

        let resp = client.execute(req).await.map_err(log_reqwest_error)?;

        tracing::trace!("Returning response from outbound request to {abs_url}");
        span.record("http.response.status_code", resp.status().as_u16());
        response_from_reqwest(resp).await
    }
}

impl http_types::Host for crate::InstanceState {
    fn convert_http_error(&mut self, err: HttpError) -> anyhow::Result<HttpError> {
        Ok(err)
    }
}

fn record_request_fields(span: &Span, req: &Request) {
    let method = match req.method {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Put => "PUT",
        Method::Delete => "DELETE",
        Method::Patch => "PATCH",
        Method::Head => "HEAD",
        Method::Options => "OPTIONS",
    };
    span.record("otel.name", method)
        .record("http.request.method", method)
        .record("url.full", req.uri.clone());
    if let Ok(uri) = req.uri.parse::<http::Uri>() {
        if let Some(authority) = uri.authority() {
            span.record("server.address", authority.host());
            if let Some(port) = authority.port() {
                span.record("server.port", port.as_u16());
            }
        }
    }
}

fn reqwest_method(m: Method) -> reqwest::Method {
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

async fn response_from_reqwest(res: reqwest::Response) -> Result<Response, HttpError> {
    let status = res.status().as_u16();

    let headers = res
        .headers()
        .into_iter()
        .map(|(key, val)| {
            Ok((
                key.to_string(),
                val.to_str()
                    .map_err(|_| {
                        tracing::error!("Non-ascii response header {key} = {val:?}");
                        HttpError::RuntimeError
                    })?
                    .to_string(),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let body = res
        .bytes()
        .await
        .map_err(|_| HttpError::RuntimeError)?
        .to_vec();

    Ok(Response {
        status,
        headers: Some(headers),
        body: Some(body),
    })
}
