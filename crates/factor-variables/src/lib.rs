mod provider;

use std::{collections::HashMap, sync::Arc};

use provider::{provider_from_toml_fn, ProviderFromToml};
use serde::Deserialize;
use spin_expressions::ProviderResolver;
use spin_factors::{
    anyhow::{self, bail, Context},
    ConfigureAppContext, Factor, FactorRuntimeConfig, InitContext, InstanceBuilders,
    PrepareContext, RuntimeFactors, SelfInstanceBuilder,
};
use spin_world::{async_trait, v1, v2::variables};

pub use provider::{MakeVariablesProvider, StaticVariables};

#[derive(Default)]
pub struct VariablesFactor {
    provider_types: HashMap<&'static str, ProviderFromToml>,
}

impl VariablesFactor {
    pub fn add_provider_type<T: MakeVariablesProvider>(
        &mut self,
        provider_type: T,
    ) -> anyhow::Result<()> {
        if self
            .provider_types
            .insert(T::RUNTIME_CONFIG_TYPE, provider_from_toml_fn(provider_type))
            .is_some()
        {
            bail!("duplicate provider type {:?}", T::RUNTIME_CONFIG_TYPE);
        }
        Ok(())
    }
}

impl Factor for VariablesFactor {
    type RuntimeConfig = RuntimeConfig;
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
            for ProviderConfig { type_, config } in runtime_config.provider_configs {
                let provider_maker = self
                    .provider_types
                    .get(type_.as_str())
                    .with_context(|| format!("unknown variables provider type {type_:?}"))?;
                let provider = provider_maker(config)?;
                resolver.add_provider(provider);
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
pub struct RuntimeConfig {
    provider_configs: Vec<ProviderConfig>,
}

impl FactorRuntimeConfig for RuntimeConfig {
    const KEY: &'static str = "variable_provider";
}

#[derive(Deserialize)]
struct ProviderConfig {
    #[serde(rename = "type")]
    type_: String,
    #[serde(flatten)]
    config: toml::Table,
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
