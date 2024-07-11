mod host;

use host::InstanceState;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factors::{
    anyhow, ConfigureAppContext, Factor, InstanceBuilders, PrepareContext, RuntimeFactors,
    SelfInstanceBuilder,
};
use std::marker::Send;
pub struct OutboundRedisFactor;

impl Factor for OutboundRedisFactor {
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::v1::redis::add_to_linker)?;
        ctx.link_bindings(spin_world::v2::redis::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: RuntimeFactors>(
        &self,
        _ctx: ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        Ok(())
    }

    fn prepare<T: RuntimeFactors>(
        &self,
        _ctx: PrepareContext<Self>,
        builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let allowed_hosts = builders
            .get_mut::<OutboundNetworkingFactor>()?
            .allowed_hosts();
        Ok(InstanceState {
            allowed_hosts,
            connections: table::Table::new(1024),
        })
    }
}

impl SelfInstanceBuilder for InstanceState {}
