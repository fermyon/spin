pub mod host_component;

use spin_app::MetadataKey;
use spin_core::async_trait;
use spin_world::llm::{self as wasi_llm};
use std::collections::HashSet;

pub use crate::host_component::LlmComponent;

pub const MODEL_ALL_MINILM_L6_V2: &str = "all-minilm-l6-v2";
pub const AI_MODELS_KEY: MetadataKey<HashSet<String>> = MetadataKey::new("ai_models");

#[async_trait]
pub trait LlmEngine: Send + Sync {
    async fn infer(
        &mut self,
        model: wasi_llm::InferencingModel,
        prompt: String,
        params: wasi_llm::InferencingParams,
    ) -> Result<wasi_llm::InferencingResult, wasi_llm::Error>;

    async fn generate_embeddings(
        &mut self,
        model: wasi_llm::EmbeddingModel,
        data: Vec<String>,
    ) -> Result<wasi_llm::EmbeddingsResult, wasi_llm::Error>;
}

pub struct LlmDispatch {
    engine: Box<dyn LlmEngine>,
    allowed_models: HashSet<String>,
}

#[async_trait]
impl wasi_llm::Host for LlmDispatch {
    async fn infer(
        &mut self,
        model: wasi_llm::InferencingModel,
        prompt: String,
        params: Option<wasi_llm::InferencingParams>,
    ) -> anyhow::Result<Result<wasi_llm::InferencingResult, wasi_llm::Error>> {
        if !self.allowed_models.contains(&model) {
            return Ok(Err(access_denied_error(&model)));
        }
        Ok(self
            .engine
            .infer(
                model,
                prompt,
                params.unwrap_or(wasi_llm::InferencingParams {
                    max_tokens: 100,
                    repeat_penalty: 1.1,
                    repeat_penalty_last_n_token_count: 64,
                    temperature: 0.8,
                    top_k: 40,
                    top_p: 0.9,
                }),
            )
            .await)
    }

    async fn generate_embeddings(
        &mut self,
        m: wasi_llm::EmbeddingModel,
        data: Vec<String>,
    ) -> anyhow::Result<Result<wasi_llm::EmbeddingsResult, wasi_llm::Error>> {
        if !self.allowed_models.contains(&m) {
            return Ok(Err(access_denied_error(&m)));
        }
        Ok(self.engine.generate_embeddings(m, data).await)
    }
}

fn access_denied_error(model: &str) -> wasi_llm::Error {
    wasi_llm::Error::InvalidInput(format!(
        "The component does not have access to use '{model}'. To give the component access, add '{model}' to the 'ai_models' key for the component in your spin.toml manifest"
    ))
}
