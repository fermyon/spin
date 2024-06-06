use spin_factors::{anyhow, Factor, InitContext, RuntimeFactors};
use wasmtime_wasi::{preview1::WasiP1Ctx, WasiCtxBuilder};

pub struct WasiPreview1Factor;

impl Factor for WasiPreview1Factor {
    type AppState = ();
    type InstancePreparer = InstancePreparer;
    type InstanceState = WasiP1Ctx;

    fn init<Factors: RuntimeFactors>(
        &mut self,
        mut ctx: InitContext<Factors, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_module_bindings(wasmtime_wasi::preview1::add_to_linker_async)
    }

    fn prepare(&self, mut preparer: InstancePreparer) -> anyhow::Result<WasiP1Ctx> {
        Ok(preparer.wasi_ctx.build_p1())
    }
}

pub struct InstancePreparer {
    wasi_ctx: WasiCtxBuilder,
}

impl Default for InstancePreparer {
    fn default() -> Self {
        Self {
            wasi_ctx: WasiCtxBuilder::new(),
        }
    }
}
