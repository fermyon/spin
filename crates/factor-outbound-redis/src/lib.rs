mod host;

use host::InstanceState;
use spin_factor_observe::ObserveContext;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factors::{
    anyhow, ConfigureAppContext, Factor, PrepareContext, RuntimeFactors, SelfInstanceBuilder,
};

/// The [`Factor`] for `fermyon:spin/outbound-redis`.
#[derive(Default)]
pub struct OutboundRedisFactor {
    _priv: (),
}

impl OutboundRedisFactor {
    pub fn new() -> Self {
        Self::default()
    }
}

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
        mut ctx: PrepareContext<T, Self>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let allowed_hosts = ctx
            .instance_builder::<OutboundNetworkingFactor>()?
            .allowed_hosts();
        let observe_context = ObserveContext::from_prepare_context(&mut ctx)?;

        Ok(InstanceState {
            allowed_hosts,
            connections: spin_resource_table::Table::new(1024),
            observe_context,
        })
    }
}

impl SelfInstanceBuilder for InstanceState {}
