use anyhow::Context;
use spin_app::DynamicHostComponent;
use spin_core::HostComponent;

use crate::OutboundMqtt;

pub struct OutboundMqttComponent;

impl HostComponent for OutboundMqttComponent {
    type Data = OutboundMqtt;
    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        spin_world::v2::mqtt::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}

impl DynamicHostComponent for OutboundMqttComponent {
    fn update_data(
        &self,
        data: &mut Self::Data,
        component: &spin_app::AppComponent,
    ) -> anyhow::Result<()> {
        let hosts = component
            .get_metadata(spin_outbound_networking::ALLOWED_HOSTS_KEY)?
            .unwrap_or_default();
        data.allowed_hosts = spin_outbound_networking::AllowedHostsConfig::parse(&hosts)
            .context("`allowed_outbound_hosts` contained an invalid url")?;
        Ok(())
    }
}
