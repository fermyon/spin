use std::net::SocketAddr;

use anyhow::Result;
use http_body_util::BodyExt;
use hyper::{Request, Response};
use spin_factor_observe::future::FutureExt;
use spin_http::body;
use spin_http::routes::RouteMatch;
use spin_world::v1::http_types;
use tracing::{instrument, Level};

use crate::{
    headers::{append_headers, prepare_request_headers},
    server::HttpExecutor,
    Body, TriggerInstanceBuilder,
};

/// An [`HttpExecutor`] that uses the `fermyon:spin/inbound-http` interface.
#[derive(Clone)]
pub struct SpinHttpExecutor;

impl HttpExecutor for SpinHttpExecutor {
    #[instrument(name = "spin_trigger_http.execute_wasm", skip_all, err(level = Level::INFO), fields(otel.name = format!("execute_wasm_component {}", route_match.component_id())))]
    async fn execute(
        &self,
        instance_builder: TriggerInstanceBuilder<'_>,
        route_match: &RouteMatch,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        let component_id = route_match.component_id();

        tracing::trace!("Executing request using the Spin executor for component {component_id}");

        let (instance, mut store) = instance_builder.instantiate(()).await?;

        let headers = prepare_request_headers(&req, route_match, client_addr)?;
        let func = instance
            .exports(&mut store)
            .instance("fermyon:spin/inbound-http")
            // Safe since we have already checked that this instance exists
            .expect("no fermyon:spin/inbound-http found")
            .typed_func::<(http_types::Request,), (http_types::Response,)>("handle-request")?;

        let (parts, body) = req.into_parts();
        let bytes = body.collect().await?.to_bytes().to_vec();

        let method = if let Some(method) = convert_method(&parts.method) {
            method
        } else {
            return Ok(Response::builder()
                .status(http::StatusCode::METHOD_NOT_ALLOWED)
                .body(body::empty())?);
        };

        // Preparing to remove the params field. We are leaving it in place for now
        // to avoid breaking the ABI, but no longer pass or accept values in it.
        // https://github.com/fermyon/spin/issues/663
        let params = vec![];

        let uri = match parts.uri.path_and_query() {
            Some(u) => u.to_string(),
            None => parts.uri.to_string(),
        };

        let req = http_types::Request {
            method,
            uri,
            headers,
            params,
            body: Some(bytes),
        };

        let observe_context = store
            .data_mut()
            .factors_instance_state()
            .observe
            .get_observe_context();

        let (resp,) = func
            .call_async(&mut store, (req,))
            .manage_wasi_observe_spans(observe_context)
            .await?;

        if resp.status < 100 || resp.status > 600 {
            tracing::error!("malformed HTTP status code");
            return Ok(Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(body::empty())?);
        };

        let mut response = http::Response::builder().status(resp.status);
        if let Some(headers) = response.headers_mut() {
            append_headers(headers, resp.headers)?;
        }

        let body = match resp.body {
            Some(b) => body::full(b.into()),
            None => body::empty(),
        };

        Ok(response.body(body)?)
    }
}

fn convert_method(m: &http::Method) -> Option<http_types::Method> {
    Some(match *m {
        http::Method::GET => http_types::Method::Get,
        http::Method::POST => http_types::Method::Post,
        http::Method::PUT => http_types::Method::Put,
        http::Method::DELETE => http_types::Method::Delete,
        http::Method::PATCH => http_types::Method::Patch,
        http::Method::HEAD => http_types::Method::Head,
        http::Method::OPTIONS => http_types::Method::Options,
        _ => return None,
    })
}
