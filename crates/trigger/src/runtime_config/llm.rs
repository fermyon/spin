pub(crate) async fn build_component(
    runtime_config: &crate::RuntimeConfig,
    use_gpu: bool,
) -> spin_llm::LlmComponent {
    spin_llm::LlmComponent::new(
        runtime_config
            .state_dir()
            .unwrap_or_default()
            .join("ai-models"),
        use_gpu,
    )
    .await
}
