mod factor_redis;

use spin_factor_outbound_networking::{OutboundAllowedHosts, OutboundNetworkingFactor};
use spin_factors::{
    anyhow, ConfigureAppContext, Factor, InstanceBuilders, PrepareContext, RuntimeFactors,
    SelfInstanceBuilder,
};

use redis::aio::Connection;
 
// use wasmtime_wasi_http::WasiHttpCtx;
pub struct OutboundRedisFactor;

impl Factor for OutboundRedisFactor {
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceState;

    fn init<T: RuntimeFactors>(
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

pub struct InstanceState {
    allowed_hosts: OutboundAllowedHosts,
    connections: table::Table<Connection>,

}


// impl Default for InstanceState {
//     fn default() -> Self {
//         Self {
//             allowed_hosts: Default::default(),
//             connections: table::Table::new(1024),
//         }
//     }
// }

impl SelfInstanceBuilder for InstanceState {}
