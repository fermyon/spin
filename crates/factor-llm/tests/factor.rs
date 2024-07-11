use std::collections::HashSet;

use factor_llm::{LlmEngine, LlmFactor};
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};
use spin_world::v1::llm::{self as v1};
use spin_world::v2::llm::{self as v2, Host};

#[derive(RuntimeFactors)]
struct TestFactors {
    llm: LlmFactor,
}

#[tokio::test]
async fn llm_works() -> anyhow::Result<()> {
    let factors = TestFactors {
        llm: LlmFactor::new(|| Box::new(FakeLLm) as _),
    };

    let env = TestEnvironment::default_manifest_extend(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        ai_models = ["llama2-chat"]
    });
    let mut state = env.build_instance_state(factors).await?;
    assert_eq!(
        &*state.llm.allowed_models,
        &["llama2-chat".to_owned()]
            .into_iter()
            .collect::<HashSet<_>>()
    );

    assert!(matches!(
        state
            .llm
            .infer("no-model".into(), "some prompt".into(), Default::default())
            .await,
        Err(v2::Error::InvalidInput(msg)) if msg.contains("The component does not have access to use")
    ));
    Ok(())
}

struct FakeLLm;

#[async_trait::async_trait]
impl LlmEngine for FakeLLm {
    async fn infer(
        &mut self,
        model: v1::InferencingModel,
        prompt: String,
        params: v2::InferencingParams,
    ) -> Result<v2::InferencingResult, v2::Error> {
        let _ = (model, prompt, params);
        todo!()
    }

    async fn generate_embeddings(
        &mut self,
        model: v2::EmbeddingModel,
        data: Vec<String>,
    ) -> Result<v2::EmbeddingsResult, v2::Error> {
        let _ = (model, data);
        todo!()
    }
}
