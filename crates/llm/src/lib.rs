use std::{
    convert::Infallible,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use llm::{
    InferenceFeedback, InferenceParameters, InferenceResponse, InferenceSessionConfig, Model,
    ModelArchitecture, ModelKVMemoryType, ModelParameters,
};

use rand::SeedableRng;
use spin_core::{async_trait, HostComponent};
use spin_world::llm::{self as wasi_llm};

#[derive(Default)]
pub struct LLmOptions {
    pub model_registry: PathBuf,
    pub use_gpu: bool,
}

pub struct LlmComponent {
    engine: LlmEngine,
}

impl HostComponent for LlmComponent {
    type Data = LlmEngine;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        spin_world::llm::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        self.engine.clone()
    }
}

impl LlmComponent {
    pub fn new(registry: PathBuf, use_gpu: bool) -> Self {
        Self {
            engine: LlmEngine::new(registry, use_gpu),
        }
    }
}

#[derive(Clone)]
pub struct LlmEngine {
    pub registry: PathBuf,
    pub use_gpu: bool,
}

impl LlmEngine {
    pub fn new(registry: PathBuf, use_gpu: bool) -> Self {
        Self { registry, use_gpu }
    }

    async fn run(&mut self, model: wasi_llm::InferencingModel, prompt: String) -> Result<String> {
        let params = ModelParameters {
            prefer_mmap: true,
            context_size: 2048,
            lora_adapters: None,
            use_gpu: self.use_gpu,
            gpu_layers: None,
            rope_overrides: None,
            n_gqa: None,
        };

        let progress_fn = |_| {};

        let model = llm::load_dynamic(
            Some(model_arch(&model)),
            &self.registry.join(&model_name(model)),
            llm::TokenizerSource::Embedded,
            params,
            progress_fn,
        )?;
        let cfg = InferenceSessionConfig {
            memory_k_type: ModelKVMemoryType::Float16,
            memory_v_type: ModelKVMemoryType::Float16,
            n_batch: 8,
            n_threads: 8,
        };

        let mut session = Model::start_session(model.as_ref(), cfg);
        let params = InferenceParameters {
            sampler: generate_sampler(),
        };
        let mut rng = rand::rngs::StdRng::from_entropy();
        let mut response = String::new();

        let res = session.infer::<Infallible>(
            model.as_ref(),
            &mut rng,
            &llm::InferenceRequest {
                prompt: prompt.as_str().into(),
                parameters: &params,
                play_back_previous_tokens: false,
                maximum_token_count: Some(75),
            },
            // OutputRequest
            &mut Default::default(),
            |r| {
                if let InferenceResponse::InferredToken(t) = r {
                    response.push_str(&t);
                }
                Ok(InferenceFeedback::Continue)
            },
        );
        let _ = res?;
        Ok(response)
    }
}

#[async_trait]
impl wasi_llm::Host for LlmEngine {
    async fn infer(
        &mut self,
        m: wasi_llm::InferencingModel,
        p: String,
        _params: Option<wasi_llm::InferencingParams>,
    ) -> Result<Result<wasi_llm::InferencingResult, wasi_llm::Error>> {
        let res = self.run(m, p).await.unwrap();
        Ok(Ok(res))
    }

    async fn generate_embeddings(
        &mut self,
        m: wasi_llm::EmbeddingModel,
        data: Vec<String>,
    ) -> Result<Result<Vec<Vec<f32>>, wasi_llm::Error>> {
        let res = self.generate_embeddings(m, data).await?.unwrap();
        Ok(Ok(res))
    }
}

// TODO: Fix these two functions
fn model_name(model: wasi_llm::InferencingModel) -> &'static str {
    match model {
        wasi_llm::InferencingModel::Llama2V70bChat => todo!(),
        wasi_llm::InferencingModel::Llama2V13bChat => "llama2-13b-chat",
        wasi_llm::InferencingModel::Llama2V7bChat => todo!(),
        wasi_llm::InferencingModel::Other(_) => todo!(),
    }
}

fn model_arch(_model: &wasi_llm::InferencingModel) -> ModelArchitecture {
    ModelArchitecture::Llama
}

// Sampling options for picking the next token in the sequence.
// We start with a default sampler, then add the inference parameters supplied by the request.
fn generate_sampler(
) -> Arc<Mutex<dyn llm::samplers::llm_samplers::types::Sampler<llm::TokenId, f32>>> {
    let mut result = llm::samplers::ConfiguredSamplers {
        // We are *not* using the default implementation for ConfiguredSamplers here
        // because the builder already sets values for parameters, which we cannot replace.
        builder: llm::samplers::llm_samplers::configure::SamplerChainBuilder::default(),
        ..Default::default()
    };

    result.builder += (
        "temperature".into(),
        llm::samplers::llm_samplers::configure::SamplerSlot::new_single(
            move || {
                Box::new(
                    llm::samplers::llm_samplers::samplers::SampleTemperature::default()
                        .temperature(0.8),
                )
            },
            Option::<llm::samplers::llm_samplers::samplers::SampleTemperature>::None,
        ),
    );
    result.builder += (
        "topp".into(),
        llm::samplers::llm_samplers::configure::SamplerSlot::new_single(
            move || Box::new(llm::samplers::llm_samplers::samplers::SampleTopP::default().p(0.9)),
            Option::<llm::samplers::llm_samplers::samplers::SampleTopP>::None,
        ),
    );
    result.builder += (
        "topk".into(),
        llm::samplers::llm_samplers::configure::SamplerSlot::new_single(
            move || Box::new(llm::samplers::llm_samplers::samplers::SampleTopK::default().k(40)),
            Option::<llm::samplers::llm_samplers::samplers::SampleTopK>::None,
        ),
    );
    result.builder += (
        "repetition".into(),
        llm::samplers::llm_samplers::configure::SamplerSlot::new_chain(
            move || {
                Box::new(
                    llm::samplers::llm_samplers::samplers::SampleRepetition::default()
                        .penalty(1.1)
                        .last_n(64),
                )
            },
            [],
        ),
    );

    result.ensure_default_slots();
    Arc::new(Mutex::new(result.builder.into_chain()))
}
