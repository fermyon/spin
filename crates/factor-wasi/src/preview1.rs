use spin_factors::{anyhow, Factor, FactorInstanceBuilder, InitContext, RuntimeFactors};
use wasmtime_wasi::{preview1::WasiP1Ctx, WasiCtxBuilder};

pub struct WasiPreview1Factor;

impl Factor for WasiPreview1Factor {
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceBuilder;

    fn init<Factors: RuntimeFactors>(
        &mut self,
        mut ctx: InitContext<Factors, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_module_bindings(wasmtime_wasi::preview1::add_to_linker_async)
    }

    fn prepare<T: RuntimeFactors>(
        _ctx: spin_factors::PrepareContext<Self>,
        _builders: &mut spin_factors::InstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        Ok(InstanceBuilder {
            wasi_ctx: WasiCtxBuilder::new(),
        })
    }
}

pub struct InstanceBuilder {
    wasi_ctx: WasiCtxBuilder,
}

impl FactorInstanceBuilder for InstanceBuilder {
    type InstanceState = WasiP1Ctx;

    fn build(self) -> anyhow::Result<Self::InstanceState> {
        let Self { mut wasi_ctx } = self;
        Ok(wasi_ctx.build_p1())
    }
}
