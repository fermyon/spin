use spin_core::HostComponent;

use crate::OutboundMqtt;

pub struct OutboundMqttComponent;

impl HostComponent for OutboundMqttComponent {
    type Data = OutboundMqtt;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        super::outbound_mqtt::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}
