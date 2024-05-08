use spin_llm_remote_http::RemoteHttpLlmEngine;
use url::Url;

#[derive(Default)]
pub struct LLmOptions {
    pub use_gpu: bool,
}

pub(crate) async fn build_component(
    runtime_config: &crate::RuntimeConfig,
    use_gpu: bool,
) -> spin_llm::LlmComponent {
    match runtime_config.llm_compute() {
        #[cfg(feature = "llm")]
        LlmComputeOpts::Spin => {
            let path = runtime_config
                .state_dir()
                .unwrap_or_default()
                .join("ai-models");
            let engine = spin_llm_local::LocalLlmEngine::new(path, use_gpu).await;
            spin_llm::LlmComponent::new(move || Box::new(engine.clone()))
        }
        #[cfg(not(feature = "llm"))]
        LlmComputeOpts::Spin => {
            let _ = use_gpu;
            spin_llm::LlmComponent::new(move || Box::new(noop::NoopLlmEngine.clone()))
        }
        LlmComputeOpts::RemoteHttp(config) => {
            tracing::info!("Using remote compute for LLMs");
            let engine =
                RemoteHttpLlmEngine::new(config.url.to_owned(), config.auth_token.to_owned());
            spin_llm::LlmComponent::new(move || Box::new(engine.clone()))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum LlmComputeOpts {
    Spin,
    RemoteHttp(RemoteHttpComputeOpts),
}

#[derive(Debug, serde::Deserialize)]
pub struct RemoteHttpComputeOpts {
    url: Url,
    auth_token: String,
}

#[cfg(not(feature = "llm"))]
mod noop {
    use async_trait::async_trait;
    use spin_llm::LlmEngine;
    use spin_world::v2::llm as wasi_llm;

    #[derive(Clone)]
    pub(super) struct NoopLlmEngine;

    #[async_trait]
    impl LlmEngine for NoopLlmEngine {
        async fn infer(
            &mut self,
            _model: wasi_llm::InferencingModel,
            _prompt: String,
            _params: wasi_llm::InferencingParams,
        ) -> Result<wasi_llm::InferencingResult, wasi_llm::Error> {
            Err(wasi_llm::Error::RuntimeError(
                "Local LLM operations are not supported in this version of Spin.".into(),
            ))
        }

        async fn generate_embeddings(
            &mut self,
            _model: wasi_llm::EmbeddingModel,
            _data: Vec<String>,
        ) -> Result<wasi_llm::EmbeddingsResult, wasi_llm::Error> {
            Err(wasi_llm::Error::RuntimeError(
                "Local LLM operations are not supported in this version of Spin.".into(),
            ))
        }
    }
}
