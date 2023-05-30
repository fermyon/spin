use anyhow::{anyhow, Result};
use reqwest::Client;
use spin_common::table::Table;
use spin_core::HostComponent;
use std::sync::Arc;
use tokio::sync::Notify;
use wit::wasi::http::types2 as types;

pub mod http;
mod poll;
mod streams;

pub mod wit {
    wasmtime::component::bindgen!({
        path: "../../wit/wasi-http",
        world: "proxy",
        async: true
    });
}

pub struct WasiCloudComponent;

impl HostComponent for WasiCloudComponent {
    type Data = WasiCloud;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        wit::Proxy::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}

#[derive(Default)]
pub struct WasiCloud {
    incoming_requests: Table<http::IncomingRequest>,
    outgoing_responses: Table<http::OutgoingResponse>,
    outgoing_requests: Table<http::OutgoingRequest>,
    incoming_responses: Table<http::IncomingResponse>,
    future_incoming_responses: Table<http::FutureIncomingResponse>,
    future_trailers: Table<http::FutureTrailers>,
    future_write_trailers_results: Table<http::FutureWriteTrailersResult>,
    fields: Table<http::Fields>,
    response_outparams: Table<http::ResponseOutparam>,
    pollables: Table<poll::Pollable>,
    input_streams: Table<streams::InputStream>,
    output_streams: Table<streams::OutputStream>,
    notify: Arc<Notify>,
    http_client: Client,
}

impl WasiCloud {
    pub fn push_incoming_request(
        &mut self,
        request: http::IncomingRequest,
    ) -> Result<types::IncomingRequest> {
        self.incoming_requests
            .push(request)
            .map_err(|()| anyhow!("table overflow"))
    }

    pub fn push_response_outparam(
        &mut self,
        outparam: http::ResponseOutparam,
    ) -> Result<types::ResponseOutparam> {
        self.response_outparams
            .push(outparam)
            .map_err(|()| anyhow!("table overflow"))
    }
}
