mod host;
// pub mod spin;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use spin_factors::{
    ConfigureAppContext, Factor, PrepareContext, RuntimeFactors, SelfInstanceBuilder,
};
use spin_locked_app::MetadataKey;

pub const ALLOWED_MODELS_KEY: MetadataKey<Vec<String>> = MetadataKey::new("ai_models");

/// The factor for LLMs.
pub struct LlmFactor {}

impl LlmFactor {
    pub fn new() -> Self {
        Self {}
    }
}

mod bindings {
    wasmtime::component::bindgen!({
        path: "wit"
    });
}

impl Factor for LlmFactor {
    type RuntimeConfig = RuntimeConfig;
    type AppState = AppState;
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(bindings::my_company::my_product::llm::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: RuntimeFactors>(
        &self,
        ctx: ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        let component_allowed_models = ctx
            .app()
            .components()
            .map(|component| {
                Ok((
                    component.id().to_string(),
                    component
                        .get_metadata(ALLOWED_MODELS_KEY)?
                        .unwrap_or_default()
                        .into_iter()
                        .collect::<HashSet<_>>()
                        .into(),
                ))
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(AppState {
            component_allowed_models,
        })
    }

    fn prepare<T: RuntimeFactors>(
        &self,
        ctx: PrepareContext<T, Self>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let allowed_models = ctx
            .app_state()
            .component_allowed_models
            .get(ctx.app_component().id())
            .cloned()
            .unwrap_or_default();

        Ok(InstanceState { allowed_models })
    }
}

/// The application state for the LLM factor.
pub struct AppState {
    component_allowed_models: HashMap<String, Arc<HashSet<String>>>,
}

/// The instance state for the LLM factor.
pub struct InstanceState {
    pub allowed_models: Arc<HashSet<String>>,
}

/// The runtime configuration for the LLM factor.
pub struct RuntimeConfig {}

impl SelfInstanceBuilder for InstanceState {}
