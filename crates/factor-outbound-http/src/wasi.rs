use http::Request;
use spin_factors::{GetFactorState, Linker, RuntimeFactors};
use wasmtime_wasi_http::{bindings::http::types::ErrorCode, WasiHttpImpl, WasiHttpView};

use crate::{wasi_2023_10_18, wasi_2023_11_10};

pub(crate) fn add_to_linker<T: RuntimeFactors>(linker: &mut Linker<T>) -> anyhow::Result<()> {
    fn type_annotate<T, U, F>(f: F) -> F
    where
        F: Fn(&mut T) -> WasiHttpImpl<MutStates<U>>,
    {
        f
    }
    let closure = type_annotate(move |data| WasiHttpImpl(MutStates { inner: data }));
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker_get_host(linker, closure)?;
    wasmtime_wasi_http::bindings::http::types::add_to_linker_get_host(linker, closure)?;

    wasi_2023_10_18::add_to_linker(linker, closure)?;
    wasi_2023_11_10::add_to_linker(linker, closure)?;

    Ok(())
}

pub(crate) struct MutStates<'a, T> {
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

    fn send_request(
        &mut self,
        _request: Request<wasmtime_wasi_http::body::HyperOutgoingBody>,
        _config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse> {
        // TODO: port implementation from spin-trigger-http
        Err(ErrorCode::HttpRequestDenied.into())
    }
}

// TODO: This is a little weird, organizationally
pub fn get_wasi_http_view<T: GetFactorState + Send>(
    instance_state: &mut T,
) -> impl WasiHttpView + '_ {
    MutStates {
        inner: instance_state,
    }
}
