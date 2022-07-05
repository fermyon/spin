use anyhow::Result;
use spin_engine::{
    host_component::{HostComponent, HostComponentsStateHandle},
    RuntimeContext,
};
use spin_manifest::CoreComponent;
use wasi_experimental_http_wasmtime::{HttpCtx as ExperimentalHttpCtx, HttpState};
use wit_bindgen_wasmtime::wasmtime::Linker;

use crate::OutboundHttp;

pub struct OutboundHttpComponent;

impl HostComponent for OutboundHttpComponent {
    type State = (OutboundHttp, ExperimentalHttpCtx);

    fn add_to_linker<T>(
        linker: &mut Linker<RuntimeContext<T>>,
        data_handle: HostComponentsStateHandle<Self::State>,
    ) -> Result<()> {
        crate::add_to_linker(linker, move |ctx| &mut data_handle.get_mut(ctx).0)?;
        HttpState::new()
            .expect("HttpState::new failed")
            .add_to_linker(linker, move |ctx| data_handle.get(ctx).1.clone())?;
        Ok(())
    }

    fn build_state(&self, component: &CoreComponent) -> Result<Self::State> {
        let outbound_http = OutboundHttp {
            allowed_hosts: Some(component.wasm.allowed_http_hosts.clone()),
        };
        let experimental_http = ExperimentalHttpCtx {
            allowed_hosts: Some(component.wasm.allowed_http_hosts.clone()),
            max_concurrent_requests: None,
        };
        Ok((outbound_http, experimental_http))
    }
}
