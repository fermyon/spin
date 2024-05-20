use spin_factors::{Factor, FactorBuilder, ModuleInitContext, PrepareContext, Result, SpinFactors};
use wasmtime_wasi::{preview1::WasiP1Ctx, WasiCtxBuilder};

pub struct WasiPreview1Factor;

impl Factor for WasiPreview1Factor {
    type Builder = Builder;
    type Data = WasiP1Ctx;

    fn module_init<Factors: SpinFactors>(&mut self, mut ctx: ModuleInitContext<Factors, Self>) -> Result<()> {
        ctx.link_bindings(wasmtime_wasi::preview1::add_to_linker_async)
    }
}

pub struct Builder {
    wasi_ctx: WasiCtxBuilder,
}

impl FactorBuilder<WasiPreview1Factor> for Builder {
    fn prepare<Factors: SpinFactors>(
        _factor: &WasiPreview1Factor,
        _ctx: PrepareContext<Factors>,
    ) -> Result<Self> {
        Ok(Self {
            wasi_ctx: WasiCtxBuilder::new(),
        })
    }

    fn build(mut self) -> Result<<WasiPreview1Factor as Factor>::Data> {
        Ok(self.wasi_ctx.build_p1())
    }
}
