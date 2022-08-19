use anyhow::Result;
use wit_bindgen_wasmtime::wasmtime::Linker;

use spin_engine::{
    host_component::{HostComponent, HostComponentsStateHandle},
    RuntimeContext,
};
use spin_manifest::CoreComponent;

use crate::OutboundHttp;

pub struct OutboundHttpComponent;

impl HostComponent for OutboundHttpComponent {
    type State = OutboundHttp;

    fn add_to_linker<T>(
        linker: &mut Linker<RuntimeContext<T>>,
        data_handle: HostComponentsStateHandle<Self::State>,
    ) -> Result<()> {
        crate::add_to_linker(linker, move |ctx| data_handle.get_mut(ctx))?;
        Ok(())
    }

    fn build_state(&self, component: &CoreComponent) -> Result<Self::State> {
        Ok(OutboundHttp {
            allowed_hosts: Some(component.wasm.allowed_http_hosts.clone()),
        })
    }
}
