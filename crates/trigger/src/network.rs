use std::sync::Arc;

use crate::TriggerHooks;

#[derive(Default)]
pub struct Network {
    resolver: Arc<spin_expressions::PreparedResolver>,
}

impl TriggerHooks for Network {
    fn app_loaded(
        &mut self,
        _app: &spin_app::App,
        _runtime_config: &crate::RuntimeConfig,
        resolver: &Arc<spin_expressions::PreparedResolver>,
    ) -> anyhow::Result<()> {
        self.resolver = resolver.clone();
        Ok(())
    }

    fn component_store_builder(
        &self,
        component: &spin_app::AppComponent,
        store_builder: &mut spin_core::StoreBuilder,
    ) -> anyhow::Result<()> {
        let hosts = component
            .get_metadata(spin_outbound_networking::ALLOWED_HOSTS_KEY)?
            .unwrap_or_default();
        let allowed_hosts =
            spin_outbound_networking::AllowedHostsConfig::parse(&hosts, &self.resolver)?;
        match allowed_hosts {
            spin_outbound_networking::AllowedHostsConfig::All => store_builder.inherit_network(),
            spin_outbound_networking::AllowedHostsConfig::SpecificHosts(configs) => {
                for config in configs {
                    if config.scheme().allows_any() {
                        match config.host() {
                            spin_outbound_networking::HostConfig::Any => {
                                store_builder.inherit_network()
                            }
                            spin_outbound_networking::HostConfig::AnySubdomain(_) => continue,
                            spin_outbound_networking::HostConfig::ToSelf => {}
                            spin_outbound_networking::HostConfig::List(hosts) => {
                                for host in hosts {
                                    let Ok(ip_net) =
                                        // Parse the host as an `IpNet` cidr block and if it fails
                                        // then try parsing again with `/32` appended to the end.
                                        host.parse().or_else(|_| format!("{host}/32").parse())
                                    else {
                                        continue;
                                    };
                                    add_ip_net(store_builder, ip_net, config.port());
                                }
                            }
                            spin_outbound_networking::HostConfig::Cidr(ip_net) => {
                                add_ip_net(store_builder, *ip_net, config.port())
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn add_ip_net(
    store_builder: &mut spin_core::StoreBuilder,
    ip_net: ipnet::IpNet,
    port: &spin_outbound_networking::PortConfig,
) {
    match port {
        spin_outbound_networking::PortConfig::Any => {
            store_builder.insert_ip_net_port_range(ip_net, 0, None);
        }
        spin_outbound_networking::PortConfig::List(ports) => {
            for port in ports {
                match port {
                    spin_outbound_networking::IndividualPortConfig::Port(p) => {
                        store_builder.insert_ip_net_port_range(ip_net, *p, p.checked_add(1));
                    }
                    spin_outbound_networking::IndividualPortConfig::Range(r) => {
                        store_builder.insert_ip_net_port_range(ip_net, r.start, Some(r.end))
                    }
                }
            }
        }
    }
}
