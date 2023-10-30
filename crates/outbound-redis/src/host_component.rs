use anyhow::Context;
use spin_app::DynamicHostComponent;
use spin_core::HostComponent;

use crate::OutboundRedis;

pub struct OutboundRedisComponent;

impl HostComponent for OutboundRedisComponent {
    type Data = OutboundRedis;
    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        spin_world::v1::redis::add_to_linker(linker, get)?;
        spin_world::v2::redis::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}

impl DynamicHostComponent for OutboundRedisComponent {
    fn update_data(
        &self,
        data: &mut Self::Data,
        component: &spin_app::AppComponent,
    ) -> anyhow::Result<()> {
        let hosts = component
            .get_metadata(spin_outbound_networking::ALLOWED_HOSTS_KEY)?
            .unwrap_or_default();
        data.allowed_hosts = hosts
            .map(|h| spin_outbound_networking::AllowedHostsConfig::parse(&h[..]))
            .transpose()
            .context("`allowed_outbound_hosts` contained an invalid url")?;
        Ok(())
    }
}
