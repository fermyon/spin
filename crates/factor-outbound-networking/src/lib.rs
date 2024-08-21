pub mod runtime_config;

use std::{collections::HashMap, sync::Arc};

use futures_util::{
    future::{BoxFuture, Shared},
    FutureExt,
};
use runtime_config::RuntimeConfig;
use spin_factor_variables::VariablesFactor;
use spin_factor_wasi::{SocketAddrUse, WasiFactor};
use spin_factors::{
    anyhow::{self, Context},
    ConfigureAppContext, Error, Factor, FactorInstanceBuilder, InstanceBuilders, PrepareContext,
    RuntimeFactors,
};
use spin_outbound_networking::{AllowedHostsConfig, ALLOWED_HOSTS_KEY};

pub use spin_outbound_networking::OutboundUrl;

pub use runtime_config::ComponentTlsConfigs;

pub type SharedFutureResult<T> = Shared<BoxFuture<'static, Result<Arc<T>, Arc<anyhow::Error>>>>;

#[derive(Default)]
pub struct OutboundNetworkingFactor {
    disallowed_host_callback: Option<DisallowedHostCallback>,
}

pub type DisallowedHostCallback = fn(scheme: &str, authority: &str);

impl OutboundNetworkingFactor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a function to be called when a request is disallowed by an
    /// instance's configured `allowed_outbound_hosts`.
    pub fn set_disallowed_host_callback(&mut self, callback: DisallowedHostCallback) {
        self.disallowed_host_callback = Some(callback);
    }
}

impl Factor for OutboundNetworkingFactor {
    type RuntimeConfig = RuntimeConfig;
    type AppState = AppState;
    type InstanceBuilder = InstanceBuilder;

    fn configure_app<T: RuntimeFactors>(
        &self,
        mut ctx: ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        // Extract allowed_outbound_hosts for all components
        let component_allowed_hosts = ctx
            .app()
            .components()
            .map(|component| {
                Ok((
                    component.id().to_string(),
                    component
                        .get_metadata(ALLOWED_HOSTS_KEY)?
                        .unwrap_or_default()
                        .into_boxed_slice()
                        .into(),
                ))
            })
            .collect::<anyhow::Result<_>>()?;

        let runtime_config = match ctx.take_runtime_config() {
            Some(cfg) => cfg,
            // The default RuntimeConfig provides default TLS client configs
            None => RuntimeConfig::new([])?,
        };

        Ok(AppState {
            component_allowed_hosts,
            runtime_config,
        })
    }

    fn prepare<T: RuntimeFactors>(
        &self,
        ctx: PrepareContext<Self>,
        builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let hosts = ctx
            .app_state()
            .component_allowed_hosts
            .get(ctx.app_component().id())
            .cloned()
            .context("missing component allowed hosts")?;
        let resolver = builders
            .get_mut::<VariablesFactor>()?
            .expression_resolver()
            .clone();
        let allowed_hosts_future = async move {
            let prepared = resolver.prepare().await?;
            AllowedHostsConfig::parse(&hosts, &prepared)
        }
        .map(|res| res.map(Arc::new).map_err(Arc::new))
        .boxed()
        .shared();

        match builders.get_mut::<WasiFactor>() {
            Ok(wasi_builder) => {
                // Update Wasi socket allowed ports
                let allowed_hosts = OutboundAllowedHosts {
                    allowed_hosts_future: allowed_hosts_future.clone(),
                    disallowed_host_callback: self.disallowed_host_callback,
                };
                wasi_builder.outbound_socket_addr_check(move |addr, addr_use| {
                    let allowed_hosts = allowed_hosts.clone();
                    async move {
                        // TODO: validate against existing spin-core behavior
                        let scheme = match addr_use {
                            SocketAddrUse::TcpBind => return false,
                            SocketAddrUse::TcpConnect => "tcp",
                            SocketAddrUse::UdpBind | SocketAddrUse::UdpConnect | SocketAddrUse::UdpOutgoingDatagram => "udp",
                        };
                        allowed_hosts.check_url(&addr.to_string(), scheme).await.unwrap_or_else(|err| {
                            // TODO: should this trap (somehow)?
                            tracing::error!(%err, "allowed_outbound_hosts variable resolution failed");
                            false
                        })
                    }
                });
            }
            Err(Error::NoSuchFactor(_)) => (), // no WasiFactor to configure; that's OK
            Err(err) => return Err(err.into()),
        }

        let component_tls_configs = ctx
            .app_state()
            .runtime_config
            .get_component_tls_configs(ctx.app_component().id());

        Ok(InstanceBuilder {
            allowed_hosts_future,
            component_tls_configs,
            disallowed_host_callback: self.disallowed_host_callback,
        })
    }
}

pub struct AppState {
    component_allowed_hosts: HashMap<String, Arc<[String]>>,
    runtime_config: RuntimeConfig,
}

pub struct InstanceBuilder {
    allowed_hosts_future: SharedFutureResult<AllowedHostsConfig>,
    component_tls_configs: ComponentTlsConfigs,
    disallowed_host_callback: Option<DisallowedHostCallback>,
}

impl InstanceBuilder {
    pub fn allowed_hosts(&self) -> OutboundAllowedHosts {
        OutboundAllowedHosts {
            allowed_hosts_future: self.allowed_hosts_future.clone(),
            disallowed_host_callback: self.disallowed_host_callback,
        }
    }

    pub fn component_tls_configs(&self) -> &ComponentTlsConfigs {
        &self.component_tls_configs
    }
}

impl FactorInstanceBuilder for InstanceBuilder {
    type InstanceState = ();

    fn build(self) -> anyhow::Result<Self::InstanceState> {
        Ok(())
    }
}

// TODO: Refactor w/ spin-outbound-networking crate to simplify
#[derive(Clone)]
pub struct OutboundAllowedHosts {
    allowed_hosts_future: SharedFutureResult<AllowedHostsConfig>,
    disallowed_host_callback: Option<DisallowedHostCallback>,
}

impl OutboundAllowedHosts {
    pub async fn resolve(&self) -> anyhow::Result<Arc<AllowedHostsConfig>> {
        self.allowed_hosts_future.clone().await.map_err(|err| {
            // TODO: better way to handle this?
            anyhow::Error::msg(err)
        })
    }

    /// Checks if the given URL is allowed by this component's
    /// `allowed_outbound_hosts`.
    pub async fn allows(&self, url: &OutboundUrl) -> anyhow::Result<bool> {
        Ok(self.resolve().await?.allows(url))
    }

    /// Report that an outbound connection has been disallowed by e.g.
    /// [`OutboundAllowedHosts::allows`] returning `false`.
    ///
    /// Calls the [`DisallowedHostCallback`] if set.
    pub fn report_disallowed_host(&self, scheme: &str, authority: &str) {
        if let Some(disallowed_host_callback) = self.disallowed_host_callback {
            disallowed_host_callback(scheme, authority);
        }
    }

    /// Checks address against allowed hosts
    ///
    /// Calls the [`DisallowedHostCallback`] if set and URL is disallowed.
    pub async fn check_url(&self, url: &str, scheme: &str) -> anyhow::Result<bool> {
        let Ok(url) = OutboundUrl::parse(url, scheme) else {
            tracing::warn!(
                "A component tried to make a request to a url that could not be parsed: {url}",
            );
            return Ok(false);
        };

        let allowed_hosts = self.resolve().await?;

        let is_allowed = allowed_hosts.allows(&url);
        if !is_allowed {
            self.report_disallowed_host(url.scheme(), &url.authority());
        }
        Ok(is_allowed)
    }
}
