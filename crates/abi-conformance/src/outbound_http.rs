use super::Context;
use anyhow::ensure;
use std::collections::HashMap;
use wasi_outbound_http::{HttpError, Request, Response, WasiOutboundHttp};
use wasmtime::{InstancePre, Store};

pub use wasi_outbound_http::add_to_linker;

wit_bindgen_wasmtime::export!("../../wit/ephemeral/wasi-outbound-http.wit");

#[derive(Default)]
pub(super) struct OutboundHttp {
    map: HashMap<String, String>,
}

impl WasiOutboundHttp for OutboundHttp {
    fn request(&mut self, req: Request) -> Result<Response, HttpError> {
        self.map
            .remove(req.uri)
            .map(|body| Response {
                status: 200,
                headers: None,
                body: Some(body.into_bytes()),
            })
            .ok_or(HttpError::InvalidUrl)
    }
}

pub(super) fn test(store: &mut Store<Context>, pre: &InstancePre<Context>) -> Result<(), String> {
    store
        .data_mut()
        .outbound_http
        .map
        .insert("http://127.0.0.1/test".into(), "Jabberwocky".into());

    super::run_command(
        store,
        pre,
        &["outbound-http", "http://127.0.0.1/test"],
        |store| {
            ensure!(
                store.data().outbound_http.map.is_empty(),
                "expected module to call `wasi-outbound-http::request` exactly once"
            );

            Ok(())
        },
    )
}
