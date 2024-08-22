use std::path::PathBuf;
use std::sync::Arc;

pub use spin_llm_local::LocalLlmEngine;

use spin_llm_remote_http::RemoteHttpLlmEngine;
use spin_world::async_trait;
use spin_world::v1::llm::{self as v1};
use spin_world::v2::llm::{self as v2};
use tokio::sync::Mutex;
use url::Url;

use crate::{LlmEngine, LlmEngineCreator, RuntimeConfig};

#[async_trait]
impl LlmEngine for LocalLlmEngine {
    async fn infer(
        &mut self,
        model: v1::InferencingModel,
        prompt: String,
        params: v2::InferencingParams,
    ) -> Result<v2::InferencingResult, v2::Error> {
        self.infer(model, prompt, params).await
    }

    async fn generate_embeddings(
        &mut self,
        model: v2::EmbeddingModel,
        data: Vec<String>,
    ) -> Result<v2::EmbeddingsResult, v2::Error> {
        self.generate_embeddings(model, data).await
    }
}

#[async_trait]
impl LlmEngine for RemoteHttpLlmEngine {
    async fn infer(
        &mut self,
        model: v1::InferencingModel,
        prompt: String,
        params: v2::InferencingParams,
    ) -> Result<v2::InferencingResult, v2::Error> {
        self.infer(model, prompt, params).await
    }

    async fn generate_embeddings(
        &mut self,
        model: v2::EmbeddingModel,
        data: Vec<String>,
    ) -> Result<v2::EmbeddingsResult, v2::Error> {
        self.generate_embeddings(model, data).await
    }
}

pub fn runtime_config_from_toml(
    table: &toml::Table,
    state_dir: PathBuf,
    use_gpu: bool,
) -> anyhow::Result<Option<RuntimeConfig>> {
    let Some(value) = table.get("llm_compute") else {
        return Ok(None);
    };
    let config: LlmCompute = value.clone().try_into()?;

    Ok(Some(RuntimeConfig {
        engine: config.into_engine(state_dir, use_gpu),
    }))
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum LlmCompute {
    Spin,
    RemoteHttp(RemoteHttpCompute),
}

impl LlmCompute {
    fn into_engine(self, state_dir: PathBuf, use_gpu: bool) -> Arc<Mutex<dyn LlmEngine>> {
        match self {
            LlmCompute::Spin => default_engine_creator(state_dir, use_gpu).create(),
            LlmCompute::RemoteHttp(config) => Arc::new(Mutex::new(RemoteHttpLlmEngine::new(
                config.url,
                config.auth_token,
            ))),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct RemoteHttpCompute {
    url: Url,
    auth_token: String,
}

/// The default engine creator for the LLM factor when used in the Spin CLI.
pub fn default_engine_creator(
    state_dir: PathBuf,
    use_gpu: bool,
) -> impl LlmEngineCreator + 'static {
    move || {
        Arc::new(Mutex::new(LocalLlmEngine::new(
            state_dir.join("ai-models"),
            use_gpu,
        ))) as _
    }
}
