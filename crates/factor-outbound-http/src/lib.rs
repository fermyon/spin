mod spin;
mod wasi;
mod wasi_2023_10_18;
mod wasi_2023_11_10;

use spin_factor_outbound_networking::{OutboundAllowedHosts, OutboundNetworkingFactor};
use spin_factors::{
    anyhow, ConfigureAppContext, Factor, InstanceBuilders, PrepareContext, RuntimeFactors,
    SelfInstanceBuilder,
};
use wasmtime_wasi_http::WasiHttpCtx;

pub struct OutboundHttpFactor;

impl Factor for OutboundHttpFactor {
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::v1::http::add_to_linker)?;
        wasi::add_to_linker::<T>(&mut ctx)?;
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
            wasi_http_ctx: WasiHttpCtx::new(),
        })
    }
}

pub struct InstanceState {
    allowed_hosts: OutboundAllowedHosts,
    wasi_http_ctx: WasiHttpCtx,
    data: Data,
}

struct Data {
    allowed_hosts: spin_factor_outbound_networking::OutboundAllowedHosts,
    origin: Option<String>,
    client_tls_opts: Option<HashMap<Authority, ParsedClientTlsOpts>>,
    // chained_handler: Option<ChainedHandler>,
}

impl SelfInstanceBuilder for InstanceState {}
