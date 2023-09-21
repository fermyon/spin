use spin_llm_local::LocalLlmEngine;
use spin_llm_remote_http::RemoteHttpLlmEngine;
use url::Url;

pub(crate) async fn build_component(
    runtime_config: &crate::RuntimeConfig,
    use_gpu: bool,
) -> spin_llm::LlmComponent {
    match runtime_config.llm_compute() {
        LlmComputeOpts::Spin => {
            let path = runtime_config
                .state_dir()
                .unwrap_or_default()
                .join("ai-models");
            let engine = LocalLlmEngine::new(path, use_gpu).await;
            spin_llm::LlmComponent::new(move || Box::new(engine.clone()))
        }
        LlmComputeOpts::RemoteHttp(config) => {
            tracing::log::info!("Using remote compute for LLMs");
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
