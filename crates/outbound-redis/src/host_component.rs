use spin_core::HostComponent;

use crate::OutboundRedis;

pub struct OutboundRedisComponent;

impl HostComponent for OutboundRedisComponent {
    type Data = OutboundRedis;
    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        spin_world::v2::redis::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}

pub struct LegacyOutboundRedisComponent;

impl HostComponent for LegacyOutboundRedisComponent {
    type Data = OutboundRedis;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        spin_world::v1::redis::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}
