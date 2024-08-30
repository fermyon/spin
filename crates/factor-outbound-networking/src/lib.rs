mod config;
pub mod runtime_config;

use std::{collections::HashMap, sync::Arc};

use config::ALLOWED_HOSTS_KEY;
use futures_util::{
    future::{BoxFuture, Shared},
    FutureExt,
};
use runtime_config::RuntimeConfig;
use spin_factor_variables::VariablesFactor;
use spin_factor_wasi::{SocketAddrUse, WasiFactor};
use spin_factors::{
    anyhow::{self, Context},
    ConfigureAppContext, Error, Factor, FactorInstanceBuilder, PrepareContext, RuntimeFactors,
};

pub use config::{
    is_service_chaining_host, parse_service_chaining_target, AllowedHostConfig, AllowedHostsConfig,
    HostConfig, OutboundUrl, SERVICE_CHAINING_DOMAIN_SUFFIX,
};

pub use runtime_config::ComponentTlsConfigs;

pub type SharedFutureResult<T> = Shared<BoxFuture<'static, Result<Arc<T>, Arc<anyhow::Error>>>>;

#[derive(Default)]
pub struct OutboundNetworkingFactor {
    disallowed_host_handler: Option<Arc<dyn DisallowedHostHandler>>,
}

impl OutboundNetworkingFactor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a handler to be called when a request is disallowed by an
    /// instance's configured `allowed_outbound_hosts`.
    pub fn set_disallowed_host_handler(&mut self, handler: impl DisallowedHostHandler + 'static) {
        self.disallowed_host_handler = Some(Arc::new(handler));
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
        mut ctx: PrepareContext<T, Self>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let hosts = ctx
            .app_state()
            .component_allowed_hosts
            .get(ctx.app_component().id())
            .cloned()
            .context("missing component allowed hosts")?;
        let resolver = ctx
            .instance_builder::<VariablesFactor>()?
            .expression_resolver()
            .clone();
        let allowed_hosts_future = async move {
            let prepared = resolver.prepare().await?;
            AllowedHostsConfig::parse(&hosts, &prepared)
        }
        .map(|res| res.map(Arc::new).map_err(Arc::new))
        .boxed()
        .shared();

        match ctx.instance_builder::<WasiFactor>() {
            Ok(wasi_builder) => {
                // Update Wasi socket allowed ports
                let allowed_hosts = OutboundAllowedHosts {
                    allowed_hosts_future: allowed_hosts_future.clone(),
                    disallowed_host_handler: self.disallowed_host_handler.clone(),
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
            disallowed_host_handler: self.disallowed_host_handler.clone(),
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
    disallowed_host_handler: Option<Arc<dyn DisallowedHostHandler>>,
}

impl InstanceBuilder {
    pub fn allowed_hosts(&self) -> OutboundAllowedHosts {
        OutboundAllowedHosts {
            allowed_hosts_future: self.allowed_hosts_future.clone(),
            disallowed_host_handler: self.disallowed_host_handler.clone(),
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
    disallowed_host_handler: Option<Arc<dyn DisallowedHostHandler>>,
}

impl OutboundAllowedHosts {
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

    /// Checks if allowed hosts permit relative requests
    ///
    /// Calls the [`DisallowedHostCallback`] if set and relative requests are
    /// disallowed.
    pub async fn check_relative_url(&self, schemes: &[&str]) -> anyhow::Result<bool> {
        let allowed_hosts = self.resolve().await?;
        let is_allowed = allowed_hosts.allows_relative_url(schemes);
        if !is_allowed {
            let scheme = schemes.first().unwrap_or(&"");
            self.report_disallowed_host(scheme, "self");
        }
        Ok(is_allowed)
    }

    async fn resolve(&self) -> anyhow::Result<Arc<AllowedHostsConfig>> {
        self.allowed_hosts_future.clone().await.map_err(|err| {
            tracing::error!("Error resolving allowed_outbound_hosts variables: {err}");
            anyhow::Error::msg(err)
        })
    }

    fn report_disallowed_host(&self, scheme: &str, authority: &str) {
        if let Some(handler) = &self.disallowed_host_handler {
            handler.handle_disallowed_host(scheme, authority);
        }
    }
}

pub trait DisallowedHostHandler: Send + Sync {
    fn handle_disallowed_host(&self, scheme: &str, authority: &str);
}

impl<F: Fn(&str, &str) + Send + Sync> DisallowedHostHandler for F {
    fn handle_disallowed_host(&self, scheme: &str, authority: &str) {
        self(scheme, authority);
    }
}
