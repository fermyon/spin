pub mod provider;
pub mod spin_cli;

use std::sync::Arc;

use serde::{de::DeserializeOwned, Deserialize};
use spin_expressions::ProviderResolver;
use spin_factors::{
    anyhow, ConfigureAppContext, Factor, FactorRuntimeConfig, InitContext, InstanceBuilders,
    PrepareContext, RuntimeFactors, SelfInstanceBuilder,
};
use spin_world::{async_trait, v1, v2::variables};

pub use provider::MakeVariablesProvider;

pub struct VariablesFactor<C> {
    provider_types: Vec<Box<dyn MakeVariablesProvider<RuntimeConfig = C>>>,
}

impl<C> Default for VariablesFactor<C> {
    fn default() -> Self {
        Self {
            provider_types: Default::default(),
        }
    }
}

impl<C> VariablesFactor<C> {
    pub fn add_provider_type<T: MakeVariablesProvider<RuntimeConfig = C>>(
        &mut self,
        provider_type: T,
    ) -> anyhow::Result<()> {
        self.provider_types.push(Box::new(provider_type) as _);
        Ok(())
    }
}

impl<C: DeserializeOwned + 'static> Factor for VariablesFactor<C> {
    type RuntimeConfig = RuntimeConfig<C>;
    type AppState = AppState;
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(&mut self, mut ctx: InitContext<T, Self>) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::v1::config::add_to_linker)?;
        ctx.link_bindings(spin_world::v2::variables::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: RuntimeFactors>(
        &self,
        mut ctx: ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        let app = ctx.app();
        let mut resolver =
            ProviderResolver::new(app.variables().map(|(key, val)| (key.clone(), val.clone())))?;

        for component in app.components() {
            resolver.add_component_variables(
                component.id(),
                component.config().map(|(k, v)| (k.into(), v.into())),
            )?;
        }

        if let Some(runtime_config) = ctx.take_runtime_config() {
            for config in runtime_config.provider_configs {
                for make_provider in self.provider_types.iter() {
                    if let Some(provider) = make_provider.make_provider(&config)? {
                        resolver.add_provider(provider);
                    }
                }
            }
        }

        Ok(AppState {
            resolver: Arc::new(resolver),
        })
    }

    fn prepare<T: RuntimeFactors>(
        &self,
        ctx: PrepareContext<Self>,
        _builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<InstanceState> {
        let component_id = ctx.app_component().id().to_string();
        let resolver = ctx.app_state().resolver.clone();
        Ok(InstanceState {
            component_id,
            resolver,
        })
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
pub struct RuntimeConfig<C> {
    provider_configs: Vec<C>,
}

impl<C: DeserializeOwned> FactorRuntimeConfig for RuntimeConfig<C> {
    const KEY: &'static str = "variable_provider";
}

pub struct AppState {
    resolver: Arc<ProviderResolver>,
}

pub struct InstanceState {
    component_id: String,
    resolver: Arc<ProviderResolver>,
}

impl InstanceState {
    pub fn resolver(&self) -> &Arc<ProviderResolver> {
        &self.resolver
    }
}

impl SelfInstanceBuilder for InstanceState {}

#[async_trait]
impl variables::Host for InstanceState {
    async fn get(&mut self, key: String) -> Result<String, variables::Error> {
        let key = spin_expressions::Key::new(&key).map_err(expressions_to_variables_err)?;
        self.resolver
            .resolve(&self.component_id, key)
            .await
            .map_err(expressions_to_variables_err)
    }

    fn convert_error(&mut self, error: variables::Error) -> anyhow::Result<variables::Error> {
        Ok(error)
    }
}

#[async_trait]
impl v1::config::Host for InstanceState {
    async fn get_config(&mut self, key: String) -> Result<String, v1::config::Error> {
        <Self as variables::Host>::get(self, key)
            .await
            .map_err(|err| match err {
                variables::Error::InvalidName(msg) => v1::config::Error::InvalidKey(msg),
                variables::Error::Undefined(msg) => v1::config::Error::Provider(msg),
                other => v1::config::Error::Other(format!("{other}")),
            })
    }

    fn convert_error(&mut self, err: v1::config::Error) -> anyhow::Result<v1::config::Error> {
        Ok(err)
    }
}

fn expressions_to_variables_err(err: spin_expressions::Error) -> variables::Error {
    use spin_expressions::Error;
    match err {
        Error::InvalidName(msg) => variables::Error::InvalidName(msg),
        Error::Undefined(msg) => variables::Error::Undefined(msg),
        Error::Provider(err) => variables::Error::Provider(err.to_string()),
        other => variables::Error::Other(format!("{other}")),
    }
}
