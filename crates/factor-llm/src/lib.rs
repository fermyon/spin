mod host;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use spin_factors::{
    ConfigureAppContext, Factor, InstanceBuilders, PrepareContext, RuntimeFactors,
    SelfInstanceBuilder,
};
use spin_locked_app::MetadataKey;
use spin_world::v1::llm::{self as v1};
use spin_world::v2::llm::{self as v2};

pub const ALLOWED_MODELS_KEY: MetadataKey<Vec<String>> = MetadataKey::new("ai_models");

pub struct LlmFactor {
    create_engine: Box<dyn Fn() -> Box<dyn LlmEngine> + Send + Sync>,
}

impl LlmFactor {
    pub fn new<F>(create_engine: F) -> Self
    where
        F: Fn() -> Box<dyn LlmEngine> + Send + Sync + 'static,
    {
        Self {
            create_engine: Box::new(create_engine),
        }
    }
}

impl Factor for LlmFactor {
    type RuntimeConfig = ();
    type AppState = AppState;
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::v1::llm::add_to_linker)?;
        ctx.link_bindings(spin_world::v2::llm::add_to_linker)?;
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
        ctx: PrepareContext<Self>,
        _builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let allowed_models = ctx
            .app_state()
            .component_allowed_models
            .get(ctx.app_component().id())
            .cloned()
            .unwrap_or_default();

        Ok(InstanceState {
            engine: (self.create_engine)(),
            allowed_models,
        })
    }
}

pub struct AppState {
    component_allowed_models: HashMap<String, Arc<HashSet<String>>>,
}

pub struct InstanceState {
    engine: Box<dyn LlmEngine>,
    pub allowed_models: Arc<HashSet<String>>,
}

impl SelfInstanceBuilder for InstanceState {}

#[async_trait]
pub trait LlmEngine: Send + Sync {
    async fn infer(
        &mut self,
        model: v1::InferencingModel,
        prompt: String,
        params: v2::InferencingParams,
    ) -> Result<v2::InferencingResult, v2::Error>;

    async fn generate_embeddings(
        &mut self,
        model: v2::EmbeddingModel,
        data: Vec<String>,
    ) -> Result<v2::EmbeddingsResult, v2::Error>;
}
