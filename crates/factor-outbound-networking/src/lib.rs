use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use futures_util::{
    future::{BoxFuture, Shared},
    FutureExt,
};
use spin_factor_variables::VariablesFactor;
use spin_factor_wasi::WasiFactor;
use spin_factors::{
    Factor, FactorInstancePreparer, InstancePreparers, PrepareContext, Result, SpinFactors,
};
use spin_outbound_networking::{AllowedHostsConfig, ALLOWED_HOSTS_KEY};

pub struct OutboundNetworkingFactor;

impl Factor for OutboundNetworkingFactor {
    type AppConfig = AppConfig;
    type InstancePreparer = InstancePreparer;
    type InstanceState = ();

    fn configure_app<Factors: SpinFactors>(
        &self,
        app: &spin_factors::App,
        _ctx: spin_factors::ConfigureAppContext<Factors>,
    ) -> Result<Self::AppConfig> {
        // Extract allowed_outbound_hosts for all components
        let component_allowed_hosts = app
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
            .collect::<Result<_>>()?;
        Ok(AppConfig {
            component_allowed_hosts,
        })
    }
}

#[derive(Default)]
pub struct AppConfig {
    component_allowed_hosts: HashMap<String, Arc<[String]>>,
}

type AllowedHostsFuture = Shared<BoxFuture<'static, Arc<anyhow::Result<AllowedHostsConfig>>>>;

pub struct InstancePreparer {
    allowed_hosts_future: AllowedHostsFuture,
}

impl FactorInstancePreparer<OutboundNetworkingFactor> for InstancePreparer {
    fn new<Factors: SpinFactors>(
        ctx: PrepareContext<OutboundNetworkingFactor>,
        mut preparers: InstancePreparers<Factors>,
    ) -> Result<Self> {
        let hosts = ctx
            .app_config()
            .component_allowed_hosts
            .get(ctx.app_component().id())
            .cloned()
            .context("missing component allowed hosts")?;
        let resolver = preparers.get_mut::<VariablesFactor>()?.resolver().clone();
        let allowed_hosts_future = async move {
            let prepared = resolver.prepare().await?;
            AllowedHostsConfig::parse(&hosts, &prepared)
        }
        .map(Arc::new)
        .boxed()
        .shared();
        // let prepared_resolver = resolver.prepare().await?;
        // let allowed_hosts = AllowedHostsConfig::parse(
        //         .context("missing component allowed hosts")?,
        //     &prepared_resolver,
        // )?;

        // Update Wasi socket allowed ports
        let wasi_preparer = preparers.get_mut::<WasiFactor>()?;
        let hosts_future = allowed_hosts_future.clone();
        wasi_preparer.outbound_socket_addr_check(move |addr| {
            let hosts_future = hosts_future.clone();
            async move {
                match &*hosts_future.await {
                    Ok(allowed_hosts) => {
                        // TODO: verify this actually works...
                        spin_outbound_networking::check_url(&addr.to_string(), "*", allowed_hosts)
                    }
                    Err(err) => {
                        // TODO: should this trap (somehow)?
                        tracing::error!(%err, "allowed_outbound_hosts variable resolution failed");
                        false
                    }
                }
            }
        });
        Ok(Self {
            allowed_hosts_future,
        })
    }

    fn prepare(self) -> Result<<OutboundNetworkingFactor as Factor>::InstanceState> {
        Ok(())
    }
}

impl InstancePreparer {
    pub async fn resolve_allowed_hosts(&self) -> Arc<anyhow::Result<AllowedHostsConfig>> {
        self.allowed_hosts_future.clone().await
    }
}
