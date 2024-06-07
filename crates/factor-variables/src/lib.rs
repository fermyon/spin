use std::sync::Arc;

use spin_expressions::ProviderResolver;
use spin_factors::{
    anyhow, ConfigureAppContext, Factor, FactorInstanceBuilder, InitContext, InstanceBuilders,
    PrepareContext, RuntimeFactors,
};
use spin_world::{async_trait, v1::config as v1_config, v2::variables};

pub struct VariablesFactor;

impl Factor for VariablesFactor {
    type RuntimeConfig = ();
    type AppState = AppState;
    type InstanceBuilder = InstanceBuilder;

    fn init<Factors: RuntimeFactors>(
        &mut self,
        mut ctx: InitContext<Factors, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(v1_config::add_to_linker)?;
        ctx.link_bindings(variables::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: RuntimeFactors>(
        &self,
        ctx: ConfigureAppContext<T, Self>,
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
        // TODO: add providers from runtime config
        Ok(AppState {
            resolver: Arc::new(resolver),
        })
    }

    fn prepare<T: RuntimeFactors>(
        ctx: PrepareContext<Self>,
        _builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<InstanceBuilder> {
        let component_id = ctx.app_component().id().to_string();
        let resolver = ctx.app_state().resolver.clone();
        Ok(InstanceBuilder {
            state: InstanceState {
                component_id,
                resolver,
            },
        })
    }
}

#[derive(Default)]
pub struct AppState {
    resolver: Arc<ProviderResolver>,
}

pub struct InstanceBuilder {
    state: InstanceState,
}

impl InstanceBuilder {
    pub fn resolver(&self) -> &Arc<ProviderResolver> {
        &self.state.resolver
    }
}

impl FactorInstanceBuilder for InstanceBuilder {
    type InstanceState = InstanceState;

    fn build(self) -> anyhow::Result<Self::InstanceState> {
        Ok(self.state)
    }
}

#[derive(Default)]
pub struct InstanceState {
    component_id: String,
    resolver: Arc<ProviderResolver>,
}

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
impl v1_config::Host for InstanceState {
    async fn get_config(&mut self, key: String) -> Result<String, v1_config::Error> {
        <Self as variables::Host>::get(self, key)
            .await
            .map_err(|err| match err {
                variables::Error::InvalidName(msg) => v1_config::Error::InvalidKey(msg),
                variables::Error::Undefined(msg) => v1_config::Error::Provider(msg),
                other => v1_config::Error::Other(format!("{other}")),
            })
    }

    fn convert_error(&mut self, err: v1_config::Error) -> anyhow::Result<v1_config::Error> {
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
