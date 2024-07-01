use spin_factors::{GetFactorState, Linker, RuntimeFactors};
use wasmtime_wasi_http::{WasiHttpImpl, WasiHttpView};

pub(crate) fn add_to_linker<T: RuntimeFactors>(linker: &mut Linker<T>) -> anyhow::Result<()> {
    fn type_annotate<T, U, F>(f: F) -> F
    where
        F: Fn(&mut T) -> WasiHttpImpl<MutStates<'_, U>>,
    {
        f
    }
    let host_getter = type_annotate(move |data| WasiHttpImpl(MutStates { inner: data }));
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker_get_host(
        linker,
        host_getter,
    )?;
    wasmtime_wasi_http::bindings::http::types::add_to_linker_get_host(linker, host_getter)?;
    Ok(())
}

struct MutStates<'a, T> {
    inner: &'a mut T,
}

impl<'a, T> WasiHttpView for MutStates<'a, T>
where
    T: GetFactorState + Send,
{
    fn ctx(&mut self) -> &mut wasmtime_wasi_http::WasiHttpCtx {
        &mut self
            .inner
            .get::<crate::OutboundHttpFactor>()
            .expect("failed to get `OutboundHttpFactor`")
            .wasi_http_ctx
    }

    fn table(&mut self) -> &mut spin_factors::wasmtime::component::ResourceTable {
        self.inner
            .get::<spin_factor_wasi::WasiFactor>()
            .expect("failed to get `WasiFactor`")
            .table()
    }
}

// TODO: This is a little weird, organizationally
pub fn get_wasi_http_view<T: RuntimeFactors>(
    instance_state: &mut T::InstanceState,
) -> impl WasiHttpView + '_ {
    MutStates {
        inner: instance_state,
    }
}
