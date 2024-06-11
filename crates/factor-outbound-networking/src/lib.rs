use std::{collections::HashMap, sync::Arc};

use futures_util::{
    future::{BoxFuture, Shared},
    FutureExt,
};
use spin_factor_variables::VariablesFactor;
use spin_factor_wasi::WasiFactor;
use spin_factors::{
    anyhow::{self, Context},
    ConfigureAppContext, Factor, FactorInstanceBuilder, InstanceBuilders, PrepareContext,
    RuntimeFactors,
};
use spin_outbound_networking::{AllowedHostsConfig, ALLOWED_HOSTS_KEY};

pub use spin_outbound_networking::OutboundUrl;

pub type SharedFutureResult<T> = Shared<BoxFuture<'static, Result<Arc<T>, Arc<anyhow::Error>>>>;

pub struct OutboundNetworkingFactor;

impl Factor for OutboundNetworkingFactor {
    type RuntimeConfig = ();
    type AppState = AppState;
    type InstanceBuilder = InstanceBuilder;

    fn configure_app<T: RuntimeFactors>(
        &self,
        ctx: ConfigureAppContext<T, Self>,
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
        Ok(AppState {
            component_allowed_hosts,
        })
    }

    fn prepare<T: RuntimeFactors>(
        ctx: PrepareContext<Self>,
        builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let hosts = ctx
            .app_state()
            .component_allowed_hosts
            .get(ctx.app_component().id())
            .cloned()
            .context("missing component allowed hosts")?;
        let resolver = builders.get_mut::<VariablesFactor>()?.resolver().clone();
        let allowed_hosts_future = async move {
            let prepared = resolver.prepare().await?;
            AllowedHostsConfig::parse(&hosts, &prepared)
        }
        .map(|res| res.map(Arc::new).map_err(Arc::new))
        .boxed()
        .shared();
        // let prepared_resolver = resolver.prepare().await?;
        // let allowed_hosts = AllowedHostsConfig::parse(
        //         .context("missing component allowed hosts")?,
        //     &prepared_resolver,
        // )?;

        // Update Wasi socket allowed ports
        let wasi_preparer = builders.get_mut::<WasiFactor>()?;
        let hosts_future = allowed_hosts_future.clone();
        wasi_preparer.outbound_socket_addr_check(move |addr| {
            let hosts_future = hosts_future.clone();
            async move {
                match hosts_future.await {
                    Ok(allowed_hosts) => {
                        // TODO: verify this actually works...
                        spin_outbound_networking::check_url(&addr.to_string(), "*", &allowed_hosts)
                    }
                    Err(err) => {
                        // TODO: should this trap (somehow)?
                        tracing::error!(%err, "allowed_outbound_hosts variable resolution failed");
                        false
                    }
                }
            }
        });
        Ok(InstanceBuilder {
            allowed_hosts_future,
        })
    }
}

pub struct AppState {
    component_allowed_hosts: HashMap<String, Arc<[String]>>,
}

pub struct InstanceBuilder {
    allowed_hosts_future: SharedFutureResult<AllowedHostsConfig>,
}

impl InstanceBuilder {
    pub fn allowed_hosts(&self) -> OutboundAllowedHosts {
        OutboundAllowedHosts {
            allowed_hosts_future: self.allowed_hosts_future.clone(),
        }
    }
}

impl FactorInstanceBuilder for InstanceBuilder {
    type InstanceState = ();

    fn build(self) -> anyhow::Result<Self::InstanceState> {
        Ok(())
    }
}

// TODO: Refactor w/ spin-outbound-networking crate to simplify
pub struct OutboundAllowedHosts {
    allowed_hosts_future: SharedFutureResult<AllowedHostsConfig>,
}

impl OutboundAllowedHosts {
    pub async fn allows(&self, url: &OutboundUrl) -> anyhow::Result<bool> {
        Ok(self.resolve().await?.allows(url))
    }

    pub async fn check_url(&self, url: &str, scheme: &str) -> anyhow::Result<bool> {
        let allowed_hosts = self.resolve().await?;
        Ok(spin_outbound_networking::check_url(
            url,
            scheme,
            &allowed_hosts,
        ))
    }

    async fn resolve(&self) -> anyhow::Result<Arc<AllowedHostsConfig>> {
        self.allowed_hosts_future.clone().await.map_err(|err| {
            // TODO: better way to handle this?
            anyhow::Error::msg(err)
        })
    }
}
