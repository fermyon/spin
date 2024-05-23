use spin_factors::{
    Factor, FactorInstancePreparer, InitContext, PrepareContext, Result, SpinFactors,
};
use wasmtime_wasi::{preview1::WasiP1Ctx, WasiCtxBuilder};

pub struct WasiPreview1Factor;

impl Factor for WasiPreview1Factor {
    type InstancePreparer = InstancePreparer;
    type InstanceState = WasiP1Ctx;

    fn init<Factors: SpinFactors>(&mut self, mut ctx: InitContext<Factors, Self>) -> Result<()> {
        ctx.link_module_bindings(wasmtime_wasi::preview1::add_to_linker_async)
    }
}

pub struct InstancePreparer {
    wasi_ctx: WasiCtxBuilder,
}

impl FactorInstancePreparer<WasiPreview1Factor> for InstancePreparer {
    fn new<Factors: SpinFactors>(
        _factor: &WasiPreview1Factor,
        _ctx: PrepareContext<Factors>,
    ) -> Result<Self> {
        Ok(Self {
            wasi_ctx: WasiCtxBuilder::new(),
        })
    }

    fn prepare(mut self) -> Result<<WasiPreview1Factor as Factor>::InstanceState> {
        Ok(self.wasi_ctx.build_p1())
    }
}
