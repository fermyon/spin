use std::sync::{Arc, Mutex};

use anyhow::Result;
use once_cell::sync::OnceCell;
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::{async_trait, config, HostComponent};

use crate::{Error, Key, Provider, Resolver};

pub struct ConfigHostComponent {
    providers: Mutex<Vec<Box<dyn Provider>>>,
    resolver: Arc<OnceCell<Resolver>>,
}

impl ConfigHostComponent {
    pub fn new(providers: Vec<Box<dyn Provider>>) -> Self {
        Self {
            providers: Mutex::new(providers),
            resolver: Default::default(),
        }
    }
}

impl HostComponent for ConfigHostComponent {
    type Data = ComponentConfig;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        config::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        ComponentConfig {
            resolver: self.resolver.clone(),
            component_id: None,
        }
    }
}

impl DynamicHostComponent for ConfigHostComponent {
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()> {
        self.resolver.get_or_try_init(|| {
            let mut resolver = Resolver::new(
                component
                    .app
                    .variables()
                    .map(|(key, var)| (key.clone(), var.clone())),
            )?;
            for component in component.app.components() {
                resolver.add_component_config(
                    component.id(),
                    component.config().map(|(k, v)| (k.into(), v.into())),
                )?;
            }
            for provider in self.providers.lock().unwrap().drain(..) {
                resolver.add_provider(provider);
            }
            Ok::<_, anyhow::Error>(resolver)
        })?;
        data.component_id = Some(component.id().to_string());
        Ok(())
    }
}

/// A component configuration interface implementation.
pub struct ComponentConfig {
    resolver: Arc<OnceCell<Resolver>>,
    component_id: Option<String>,
}

#[async_trait]
impl config::Host for ComponentConfig {
    async fn get_config(&mut self, key: String) -> Result<Result<String, config::Error>> {
        Ok(async {
            // Set by DynamicHostComponent::update_data
            let component_id = self.component_id.as_deref().unwrap();
            let key = Key::new(&key)?;
            Ok(self
                .resolver
                .get()
                .unwrap()
                .resolve(component_id, key)
                .await?)
        }
        .await)
    }
}

impl From<Error> for config::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::InvalidKey(msg) => Self::InvalidKey(msg),
            Error::InvalidSchema(msg) => Self::InvalidSchema(msg),
            Error::Provider(msg) => Self::Provider(msg.to_string()),
            other => Self::Other(format!("{}", other)),
        }
    }
}
