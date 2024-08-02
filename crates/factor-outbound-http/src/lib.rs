mod spin;
mod wasi;
pub mod wasi_2023_10_18;
pub mod wasi_2023_11_10;

use anyhow::Context;
use http::{
    uri::{Authority, Parts, PathAndQuery, Scheme},
    HeaderValue, Uri,
};
use spin_factor_outbound_networking::{
    ComponentTlsConfigs, OutboundAllowedHosts, OutboundNetworkingFactor,
};
use spin_factors::{
    anyhow, ConfigureAppContext, Factor, InstanceBuilders, PrepareContext, RuntimeFactors,
    SelfInstanceBuilder,
};
use wasmtime_wasi_http::WasiHttpCtx;

pub use wasmtime_wasi_http::{
    body::HyperOutgoingBody,
    types::{HostFutureIncomingResponse, OutgoingRequestConfig},
    HttpResult,
};

pub struct OutboundHttpFactor;

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
        _ctx: PrepareContext<Self>,
        builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let outbound_networking = builders.get_mut::<OutboundNetworkingFactor>()?;
        let allowed_hosts = outbound_networking.allowed_hosts();
        let component_tls_configs = outbound_networking.component_tls_configs().clone();
        Ok(InstanceState {
            wasi_http_ctx: WasiHttpCtx::new(),
            allowed_hosts,
            component_tls_configs,
            request_interceptor: None,
        })
    }
}

pub struct InstanceState {
    wasi_http_ctx: WasiHttpCtx,
    allowed_hosts: OutboundAllowedHosts,
    component_tls_configs: ComponentTlsConfigs,
    request_interceptor: Option<Box<dyn OutboundHttpInterceptor>>,
}

impl InstanceState {
    pub fn set_request_interceptor(
        &mut self,
        interceptor: impl OutboundHttpInterceptor + 'static,
    ) -> anyhow::Result<()> {
        if self.request_interceptor.is_some() {
            anyhow::bail!("set_request_interceptor can only be called once");
        }
        self.request_interceptor = Some(Box::new(interceptor));
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

pub trait OutboundHttpInterceptor: Send + Sync {
    /// Intercept an outgoing HTTP request.
    ///
    /// If this method returns `None`, the (possibly updated) request and config
    /// will be passed on to the default outgoing request handler.
    ///
    /// If this method returns `Some(...)`, the inner result will be returned as
    /// the result of the request, bypassing the default handler.
    fn intercept(&self, intercepted: Intercepted)
        -> Option<HttpResult<HostFutureIncomingResponse>>;
}

pub struct Intercepted<'a> {
    pub request: &'a mut Request,
    pub config: &'a mut OutgoingRequestConfig,
}
