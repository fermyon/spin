use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use spin_factor_wasi::WasiFactor;
use spin_factors::{Factor, FactorInstancePreparer, Result, SpinFactors};
use spin_outbound_networking::{AllowedHostsConfig, HostConfig, PortConfig, ALLOWED_HOSTS_KEY};

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
        let mut cfg = AppConfig::default();
        // TODO: resolve resolver resolution
        let resolver = Default::default();
        for component in app.components() {
            if let Some(hosts) = component.get_metadata(ALLOWED_HOSTS_KEY)? {
                let allowed_hosts = AllowedHostsConfig::parse(&hosts, &resolver)?;
                cfg.component_allowed_hosts
                    .insert(component.id().to_string(), Arc::new(allowed_hosts));
            }
        }
        Ok(cfg)
    }
}

#[derive(Default)]
pub struct AppConfig {
    component_allowed_hosts: HashMap<String, Arc<AllowedHostsConfig>>,
}

pub struct InstancePreparer {
    allowed_hosts: Arc<AllowedHostsConfig>,
}

impl InstancePreparer {
    pub fn allowed_hosts(&self) -> &Arc<AllowedHostsConfig> {
        &self.allowed_hosts
    }
}

impl FactorInstancePreparer<OutboundNetworkingFactor> for InstancePreparer {
    fn new<Factors: SpinFactors>(
        _factor: &OutboundNetworkingFactor,
        app_component: &spin_factors::AppComponent,
        mut ctx: spin_factors::PrepareContext<Factors>,
    ) -> Result<Self> {
        let allowed_hosts = ctx
            .app_config::<OutboundNetworkingFactor>()?
            .component_allowed_hosts
            .get(app_component.id())
            .context("missing component")?
            .clone();

        // Update Wasi socket allowed ports
        let wasi_preparer = ctx.instance_preparer_mut::<WasiFactor>()?;
        match &*allowed_hosts {
            AllowedHostsConfig::All => wasi_preparer.inherit_network(),
            AllowedHostsConfig::SpecificHosts(configs) => {
                for config in configs {
                    if config.scheme().allows_any() {
                        match (config.host(), config.port()) {
                            (HostConfig::Cidr(ip_net), PortConfig::Any) => {
                                wasi_preparer.socket_allow_ports(*ip_net, 0, None)
                            }
                            _ => todo!(), // TODO: complete and validate against existing Network TriggerHooks
                        }
                    }
                }
            }
        }

        Ok(Self { allowed_hosts })
    }

    fn prepare(self) -> Result<<OutboundNetworkingFactor as Factor>::InstanceState> {
        Ok(())
    }
}
