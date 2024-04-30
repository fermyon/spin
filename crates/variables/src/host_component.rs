use std::sync::{Arc, Mutex};

use anyhow::Result;
use once_cell::sync::OnceCell;
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::{async_trait, HostComponent};
use spin_world::v1::config::Error as V1ConfigError;
use spin_world::v2::variables;

use spin_expressions::{Error, Key, Provider, ProviderResolver};

pub struct VariablesHostComponent {
    providers: Mutex<Vec<Box<dyn Provider>>>,
    resolver: Arc<OnceCell<ProviderResolver>>,
}

impl VariablesHostComponent {
    pub fn new(providers: Vec<Box<dyn Provider>>) -> Self {
        Self {
            providers: Mutex::new(providers),
            resolver: Default::default(),
        }
    }
}

impl HostComponent for VariablesHostComponent {
    type Data = ComponentVariables;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        spin_world::v1::config::add_to_linker(linker, get)?;
        variables::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        ComponentVariables {
            resolver: self.resolver.clone(),
            component_id: None,
        }
    }
}

impl DynamicHostComponent for VariablesHostComponent {
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()> {
        self.resolver.get_or_try_init(|| {
            make_resolver(component.app, self.providers.lock().unwrap().drain(..))
        })?;
        data.component_id = Some(component.id().to_string());
        Ok(())
    }
}

pub fn make_resolver(
    app: &spin_app::App,
    providers: impl IntoIterator<Item = Box<dyn Provider>>,
) -> anyhow::Result<ProviderResolver> {
    let mut resolver =
        ProviderResolver::new(app.variables().map(|(key, var)| (key.clone(), var.clone())))?;
    for component in app.components() {
        resolver.add_component_variables(
            component.id(),
            component.config().map(|(k, v)| (k.into(), v.into())),
        )?;
    }
    for provider in providers {
        resolver.add_provider(provider);
    }
    Ok(resolver)
}

/// A component variables interface implementation.
pub struct ComponentVariables {
    resolver: Arc<OnceCell<ProviderResolver>>,
    component_id: Option<String>,
}

#[async_trait]
impl variables::Host for ComponentVariables {
    async fn get(&mut self, key: String) -> Result<String, variables::Error> {
        // Set by DynamicHostComponent::update_data
        let component_id = self.component_id.as_deref().unwrap();
        let key = Key::new(&key).map_err(as_wit)?;
        self.resolver
            .get()
            .unwrap()
            .resolve(component_id, key)
            .await
            .map_err(as_wit)
    }

    fn convert_error(&mut self, error: variables::Error) -> Result<variables::Error> {
        Ok(error)
    }
}

#[async_trait]
impl spin_world::v1::config::Host for ComponentVariables {
    async fn get_config(&mut self, key: String) -> Result<String, V1ConfigError> {
        <Self as variables::Host>::get(self, key)
            .await
            .map_err(|err| match err {
                variables::Error::InvalidName(msg) => V1ConfigError::InvalidKey(msg),
                variables::Error::Undefined(msg) => V1ConfigError::Provider(msg),
                other => V1ConfigError::Other(format!("{other}")),
            })
    }

    fn convert_error(&mut self, error: V1ConfigError) -> Result<V1ConfigError> {
        Ok(error)
    }
}

fn as_wit(err: Error) -> variables::Error {
    match err {
        Error::InvalidName(msg) => variables::Error::InvalidName(msg),
        Error::Undefined(msg) => variables::Error::Undefined(msg),
        Error::Provider(err) => variables::Error::Provider(err.to_string()),
        other => variables::Error::Other(format!("{other}")),
    }
}
