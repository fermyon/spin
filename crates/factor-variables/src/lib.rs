mod host;
pub mod runtime_config;
pub mod spin_cli;

use std::sync::Arc;

use runtime_config::RuntimeConfig;
use spin_expressions::{ProviderResolver as ExpressionResolver, Template};
use spin_factors::{
    anyhow, ConfigureAppContext, Factor, InitContext, PrepareContext, PreparedInstanceBuilders,
    RuntimeFactors, SelfInstanceBuilder,
};

/// A factor for providing variables to components.
#[derive(Default)]
pub struct VariablesFactor {
    _priv: (),
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
        let mut expression_resolver =
            ExpressionResolver::new(app.variables().map(|(key, val)| (key.clone(), val.clone())))?;

        for component in app.components() {
            expression_resolver.add_component_variables(
                component.id(),
                component.config().map(|(k, v)| (k.into(), v.into())),
            )?;
        }

        for provider in ctx.take_runtime_config().unwrap_or_default() {
            expression_resolver.add_provider(provider);
        }

        Ok(AppState {
            expression_resolver: Arc::new(expression_resolver),
        })
    }

    fn prepare<T: RuntimeFactors>(
        &self,
        ctx: PrepareContext<Self>,
        _builders: &mut PreparedInstanceBuilders<T>,
    ) -> anyhow::Result<InstanceState> {
        let component_id = ctx.app_component().id().to_string();
        let expression_resolver = ctx.app_state().expression_resolver.clone();
        Ok(InstanceState {
            component_id,
            expression_resolver,
        })
    }
}

pub struct AppState {
    expression_resolver: Arc<ExpressionResolver>,
}

impl AppState {
    pub async fn resolve_expression(
        &self,
        expr: impl Into<Box<str>>,
    ) -> spin_expressions::Result<String> {
        let template = Template::new(expr)?;
        self.expression_resolver.resolve_template(&template).await
    }
}

pub struct InstanceState {
    component_id: String,
    expression_resolver: Arc<ExpressionResolver>,
}

impl InstanceState {
    pub fn expression_resolver(&self) -> &Arc<ExpressionResolver> {
        &self.expression_resolver
    }
}

impl SelfInstanceBuilder for InstanceState {}
