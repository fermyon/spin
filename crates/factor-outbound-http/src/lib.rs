mod spin;
mod wasi;
pub mod wasi_2023_10_18;
pub mod wasi_2023_11_10;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use http::{
    uri::{Authority, Parts, PathAndQuery, Scheme},
    HeaderValue, Uri,
};
use spin_factor_outbound_networking::{
    ComponentTlsConfigs, OutboundAllowedHosts, OutboundNetworkingFactor,
};
use spin_factors::{
    anyhow, ConfigureAppContext, Factor, PrepareContext, RuntimeFactors, SelfInstanceBuilder,
};
use spin_world::async_trait;
use wasmtime_wasi_http::{types::IncomingResponse, WasiHttpCtx};

pub use wasmtime_wasi_http::{
    body::HyperOutgoingBody,
    types::{HostFutureIncomingResponse, OutgoingRequestConfig},
    HttpResult,
};

pub struct OutboundHttpFactor {
    allow_private_ips: bool,
}

impl OutboundHttpFactor {
    /// Create a new OutboundHttpFactor.
    ///
    /// If `allow_private_ips` is true, requests to private IP addresses will be allowed.
    pub fn new(allow_private_ips: bool) -> Self {
        Self { allow_private_ips }
    }
}

impl Default for OutboundHttpFactor {
    fn default() -> Self {
        Self {
            allow_private_ips: true,
        }
    }
}

impl Factor for OutboundHttpFactor {
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::v1::http::add_to_linker)?;
        wasi::add_to_linker::<T>(&mut ctx)?;
        Ok(())
    }

    fn configure_app<T: RuntimeFactors>(
        &self,
        _ctx: ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        Ok(())
    }

    fn prepare<T: RuntimeFactors>(
        &self,
        mut ctx: PrepareContext<T, Self>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let outbound_networking = ctx.instance_builder::<OutboundNetworkingFactor>()?;
        let allowed_hosts = outbound_networking.allowed_hosts();
        let component_tls_configs = outbound_networking.component_tls_configs().clone();
        Ok(InstanceState {
            wasi_http_ctx: WasiHttpCtx::new(),
            allowed_hosts,
            allow_private_ips: self.allow_private_ips,
            component_tls_configs,
            self_request_origin: None,
            request_interceptor: None,
            spin_http_client: None,
        })
    }
}

pub struct InstanceState {
    wasi_http_ctx: WasiHttpCtx,
    allowed_hosts: OutboundAllowedHosts,
    allow_private_ips: bool,
    component_tls_configs: ComponentTlsConfigs,
    self_request_origin: Option<SelfRequestOrigin>,
    request_interceptor: Option<Arc<dyn OutboundHttpInterceptor>>,
    // Connection-pooling client for 'fermyon:spin/http' interface
    spin_http_client: Option<reqwest::Client>,
}

impl InstanceState {
    /// Sets the [`SelfRequestOrigin`] for this instance.
    ///
    /// This is used to handle outbound requests to relative URLs. If unset,
    /// those requests will fail.
    pub fn set_self_request_origin(&mut self, origin: SelfRequestOrigin) {
        self.self_request_origin = Some(origin);
    }

    /// Sets a [`OutboundHttpInterceptor`] for this instance.
    ///
    /// Returns an error if it has already been called for this instance.
    pub fn set_request_interceptor(
        &mut self,
        interceptor: impl OutboundHttpInterceptor + 'static,
    ) -> anyhow::Result<()> {
        if self.request_interceptor.is_some() {
            anyhow::bail!("set_request_interceptor can only be called once");
        }
        self.request_interceptor = Some(Arc::new(interceptor));
        Ok(())
    }
}

impl SelfInstanceBuilder for InstanceState {}

pub type Request = http::Request<wasmtime_wasi_http::body::HyperOutgoingBody>;

/// SelfRequestOrigin indicates the base URI to use for "self" requests.
///
/// This is meant to be set on [`Request::extensions_mut`] in appropriate
/// contexts such as an incoming request handler.
#[derive(Clone, Debug)]
pub struct SelfRequestOrigin {
    pub scheme: Scheme,
    pub authority: Authority,
}

impl SelfRequestOrigin {
    pub fn create(scheme: Scheme, addr: &SocketAddr) -> anyhow::Result<Self> {
        Ok(SelfRequestOrigin {
            scheme,
            authority: addr
                .to_string()
                .parse()
                .with_context(|| format!("address '{addr}' is not a valid authority"))?,
        })
    }

    pub fn from_uri(uri: &Uri) -> anyhow::Result<Self> {
        Ok(Self {
            scheme: uri.scheme().context("URI missing scheme")?.clone(),
            authority: uri.authority().context("URI missing authority")?.clone(),
        })
    }

    fn into_uri(self, path_and_query: Option<PathAndQuery>) -> Uri {
        let mut parts = Parts::default();
        parts.scheme = Some(self.scheme);
        parts.authority = Some(self.authority);
        parts.path_and_query = path_and_query;
        Uri::from_parts(parts).unwrap()
    }

    fn use_tls(&self) -> bool {
        self.scheme == Scheme::HTTPS
    }

    fn host_header(&self) -> HeaderValue {
        HeaderValue::from_str(self.authority.as_str()).unwrap()
    }
}

impl std::fmt::Display for SelfRequestOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}://{}", self.scheme, self.authority)
    }
}

/// An outbound HTTP request interceptor to be used with
/// [`InstanceState::set_request_interceptor`].
#[async_trait]
pub trait OutboundHttpInterceptor: Send + Sync {
    /// Intercept an outgoing HTTP request.
    ///
    /// If this method returns [`InterceptedResponse::Continue`], the (possibly
    /// updated) request and config will be passed on to the default outgoing
    /// request handler.
    ///
    /// If this method returns [`InterceptedResponse::Intercepted`], the inner
    /// result will be returned as the result of the request, bypassing the
    /// default handler. The `request` will also be dropped immediately.
    async fn intercept(
        &self,
        request: &mut Request,
        config: &mut OutgoingRequestConfig,
    ) -> HttpResult<InterceptOutcome>;
}

/// The type returned by an [`OutboundHttpInterceptor`].
pub enum InterceptOutcome {
    /// The intercepted request will be passed on to the default outgoing
    /// request handler.
    Continue,
    /// The given result will be returned as the result of the intercepted
    /// request, bypassing the default handler.
    Complete(IncomingResponse),
}
