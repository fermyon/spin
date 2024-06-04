use spin_factors::{
    anyhow, Factor, FactorInstancePreparer, InitContext, InstancePreparers, PrepareContext,
    SpinFactors,
};
use wasmtime_wasi::{preview1::WasiP1Ctx, WasiCtxBuilder};

pub struct WasiPreview1Factor;

impl Factor for WasiPreview1Factor {
    type AppConfig = ();
    type InstancePreparer = InstancePreparer;
    type InstanceState = WasiP1Ctx;

    fn init<Factors: SpinFactors>(
        &mut self,
        mut ctx: InitContext<Factors, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_module_bindings(wasmtime_wasi::preview1::add_to_linker_async)
    }
}

pub struct InstancePreparer {
    wasi_ctx: WasiCtxBuilder,
}

impl FactorInstancePreparer<WasiPreview1Factor> for InstancePreparer {
    fn new<Factors: SpinFactors>(
        _ctx: PrepareContext<WasiPreview1Factor>,
        _preparers: InstancePreparers<Factors>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            wasi_ctx: WasiCtxBuilder::new(),
        })
    }

    fn prepare(mut self) -> anyhow::Result<WasiP1Ctx> {
        Ok(self.wasi_ctx.build_p1())
    }
}
