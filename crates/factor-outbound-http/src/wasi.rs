use anyhow::Context;
use spin_factors::{Linker, RuntimeFactors};
use wasmtime_wasi_http::{WasiHttpImpl, WasiHttpView};

pub(crate) fn add_to_linker<T: RuntimeFactors>(linker: &mut Linker<T>) -> anyhow::Result<()> {
    fn type_annotate<T, F>(f: F) -> F
    where
        F: Fn(&mut T) -> WasiHttpImpl<MutStates>,
    {
        f
    }
    let wasi_and_http_getter =
        T::instance_state_getter2::<spin_factor_wasi::WasiFactor, crate::OutboundHttpFactor>()
            .context("failed to get WasiFactor")?;
    let host_getter = type_annotate(move |data| {
        let (wasi, http) = wasi_and_http_getter.get_states(data);
        WasiHttpImpl(MutStates { http, wasi })
    });
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker_get_host(
        linker,
        host_getter,
    )?;
    wasmtime_wasi_http::bindings::http::types::add_to_linker_get_host(linker, host_getter)?;
    Ok(())
}

struct MutStates<'a> {
    http: &'a mut crate::InstanceState,
    wasi: &'a mut spin_factor_wasi::InstanceState,
}

impl<'a> WasiHttpView for MutStates<'a> {
    fn ctx(&mut self) -> &mut wasmtime_wasi_http::WasiHttpCtx {
        &mut self.http.wasi_http_ctx
    }

    fn table(&mut self) -> &mut spin_factors::wasmtime::component::ResourceTable {
        self.wasi.table()
    }
}
