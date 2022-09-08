use std::sync::Arc;

use spin_engine::host_component::HostComponent;
use spin_manifest::CoreComponent;
use wit_bindgen_wasmtime::async_trait;

use crate::{Error, Key, Resolver};

mod wit {
    wit_bindgen_wasmtime::export!({paths: ["../../wit/ephemeral/spin-config.wit"], async: *});
}

pub struct ConfigHostComponent {
    resolver: Arc<Resolver>,
}

impl ConfigHostComponent {
    pub fn new(resolver: Resolver) -> Self {
        Self {
            resolver: Arc::new(resolver),
        }
    }
}

impl HostComponent for ConfigHostComponent {
    type State = ComponentConfig;

    fn add_to_linker<T: Send>(
        linker: &mut wit_bindgen_wasmtime::wasmtime::Linker<spin_engine::RuntimeContext<T>>,
        state_handle: spin_engine::host_component::HostComponentsStateHandle<Self::State>,
    ) -> anyhow::Result<()> {
        wit::spin_config::add_to_linker(linker, move |ctx| state_handle.get_mut(ctx))
    }

    fn build_state(&self, component: &CoreComponent) -> anyhow::Result<Self::State> {
        Ok(ComponentConfig {
            component_id: component.id.clone(),
            resolver: self.resolver.clone(),
        })
    }
}

/// A component configuration interface implementation.
pub struct ComponentConfig {
    component_id: String,
    resolver: Arc<Resolver>,
}

#[async_trait]
impl wit::spin_config::SpinConfig for ComponentConfig {
    async fn get_config(&mut self, key: &str) -> Result<String, wit::spin_config::Error> {
        let key = Key::new(key)?;
        Ok(self.resolver.resolve(&self.component_id, key).await?)
    }
}

impl From<Error> for wit::spin_config::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::InvalidKey(msg) => Self::InvalidKey(msg),
            Error::InvalidSchema(msg) => Self::InvalidSchema(msg),
            Error::Provider(msg) => Self::Provider(msg.to_string()),
            other => Self::Other(format!("{}", other)),
        }
    }
}
