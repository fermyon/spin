pub mod client;
mod host;

use client::Client;
use spin_factor_outbound_networking::{OutboundAllowedHosts, OutboundNetworkingFactor};
use spin_factors::{
    anyhow, ConfigureAppContext, Factor, PrepareContext, PreparedInstanceBuilders, RuntimeFactors,
    SelfInstanceBuilder,
};
use tokio_postgres::Client as PgClient;

pub struct OutboundPgFactor<C = PgClient> {
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Send + Sync + Client + 'static> Factor for OutboundPgFactor<C> {
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceState<C>;

    fn init<T: Send + 'static>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::v1::postgres::add_to_linker)?;
        ctx.link_bindings(spin_world::v2::postgres::add_to_linker)?;
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
        builders: &mut PreparedInstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let allowed_hosts = builders
            .get_mut::<OutboundNetworkingFactor>()?
            .allowed_hosts();
        Ok(InstanceState {
            allowed_hosts,
            connections: Default::default(),
        })
    }
}

impl<C> Default for OutboundPgFactor<C> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<C> OutboundPgFactor<C> {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct InstanceState<C> {
    allowed_hosts: OutboundAllowedHosts,
    connections: table::Table<C>,
}

impl<C: Send + 'static> SelfInstanceBuilder for InstanceState<C> {}
