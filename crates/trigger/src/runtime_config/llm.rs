use spin_llm::LlmEngine;

pub(crate) async fn build_component(
    runtime_config: &crate::RuntimeConfig,
    use_gpu: bool,
) -> spin_llm::LlmComponent {
    let path = runtime_config
        .state_dir()
        .unwrap_or_default()
        .join("ai-models");
    let mut engine = spin_llm::LocalLlmEngine::new(path, use_gpu);
    engine.init().await;
    spin_llm::LlmComponent::new(move || Box::new(engine.clone()))
}
