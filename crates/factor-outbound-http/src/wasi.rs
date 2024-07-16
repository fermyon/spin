use http::Request;
use spin_factors::{
    wasmtime::component::ResourceTable, RuntimeFactors, RuntimeFactorsInstanceState,
};
use wasmtime_wasi_http::{
    bindings::http::types::ErrorCode, WasiHttpCtx, WasiHttpImpl, WasiHttpView,
};

use crate::{wasi_2023_10_18, wasi_2023_11_10, OutboundHttpFactor};

pub(crate) fn add_to_linker<T: RuntimeFactors>(
    ctx: &mut spin_factors::InitContext<T, OutboundHttpFactor>,
) -> anyhow::Result<()> {
    fn type_annotate<T, F>(f: F) -> F
    where
        F: Fn(&mut T) -> WasiHttpImpl<WasiHttpImplInner>,
    {
        f
    }
    let get_data_with_table = ctx.get_data_with_table_fn();
    let closure = type_annotate(move |data| {
        let (state, table) = get_data_with_table(data);
        WasiHttpImpl(WasiHttpImplInner {
            ctx: &mut state.wasi_http_ctx,
            table,
        })
    });
    let linker = ctx.linker();
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker_get_host(linker, closure)?;
    wasmtime_wasi_http::bindings::http::types::add_to_linker_get_host(linker, closure)?;

    wasi_2023_10_18::add_to_linker(linker, closure)?;
    wasi_2023_11_10::add_to_linker(linker, closure)?;

    Ok(())
}

impl OutboundHttpFactor {
    pub fn get_wasi_http_impl(
        runtime_instance_state: &mut impl RuntimeFactorsInstanceState,
    ) -> Option<WasiHttpImpl<impl WasiHttpView + '_>> {
        let (state, table) = runtime_instance_state.get_with_table::<OutboundHttpFactor>()?;
        Some(WasiHttpImpl(WasiHttpImplInner {
            ctx: &mut state.wasi_http_ctx,
            table,
        }))
    }
}

pub(crate) struct WasiHttpImplInner<'a> {
    ctx: &'a mut WasiHttpCtx,
    table: &'a mut ResourceTable,
}

impl<'a> WasiHttpView for WasiHttpImplInner<'a> {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        self.ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        self.table
    }

    fn send_request(
        &mut self,
        _request: Request<wasmtime_wasi_http::body::HyperOutgoingBody>,
        _config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse> {
        // TODO: port implementation from spin-trigger-http
        Err(ErrorCode::HttpRequestDenied.into())
    }
}
