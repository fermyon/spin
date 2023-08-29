mod bert;

use anyhow::Context;
use bert::{BertModel, Config};
use candle::DType;
use candle_nn::VarBuilder;
use llm::{
    InferenceFeedback, InferenceParameters, InferenceResponse, InferenceSessionConfig, Model,
    ModelArchitecture, ModelKVMemoryType, ModelParameters,
};
use rand::SeedableRng;
use spin_core::{async_trait, HostComponent};
use spin_world::llm::{self as wasi_llm};
use std::{
    convert::Infallible,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokenizers::PaddingParams;

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

    async fn run(
        &mut self,
        model: wasi_llm::InferencingModel,
        prompt: String,
    ) -> Result<String, wasi_llm::Error> {
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
            Some(model_arch(&model)?),
            &self.registry.join(&model_name(model)?),
            llm::TokenizerSource::Embedded,
            params,
            progress_fn,
        )
        .map_err(|e| {
            wasi_llm::Error::RuntimeError(format!("Failed to load model from model registry: {e}"))
        })?;
        let cfg = InferenceSessionConfig {
            memory_k_type: ModelKVMemoryType::Float16,
            memory_v_type: ModelKVMemoryType::Float16,
            n_batch: 8,
            n_threads: num_cpus::get(),
        };

        let mut session = Model::start_session(model.as_ref(), cfg);
        let params = InferenceParameters {
            sampler: generate_sampler(),
        };
        let mut rng = rand::rngs::StdRng::from_entropy();
        let mut response = String::new();

        #[cfg(debug_assertions)]
        {
            terminal::warn!(
                "\
                This is a debug build - running inference might be prohibitively slow\n\
                You may want to consider switching to the release build"
            )
        }
        let res = session.infer::<Infallible>(
            model.as_ref(),
            &mut rng,
            &llm::InferenceRequest {
                prompt: prompt.as_str().into(),
                parameters: &params,
                play_back_previous_tokens: false,
                maximum_token_count: Some(75),
            },
            &mut Default::default(),
            |r| {
                if let InferenceResponse::InferredToken(t) = r {
                    response.push_str(&t);
                }
                Ok(InferenceFeedback::Continue)
            },
        );
        let _ = res.map_err(|e| {
            wasi_llm::Error::RuntimeError(format!("Failure ocurred during inferencing: {e}"))
        })?;
        Ok(response)
    }

    async fn generate_embeddings(
        &mut self,
        _model: wasi_llm::EmbeddingModel,
        data: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, wasi_llm::Error> {
        generate_embeddings(
            data,
            &self.registry.join("tokenizer.json"),
            &self.registry.join("model.safetensor"),
        )
        .await
        .map_err(|e| wasi_llm::Error::RuntimeError(format!("Error generating embeddings: {e}")))
    }
}

async fn generate_embeddings(
    data: Vec<String>,
    tokenizer_file: &Path,
    model_file: &Path,
) -> anyhow::Result<Vec<Vec<f32>>> {
    let n_sentences = data.len();
    let mut tokenizer = tokenizers::Tokenizer::from_file(tokenizer_file)
        .map_err(|e| anyhow::anyhow!("failed to read tokenizer file {tokenizer_file:?}: {e}"))?;
    // This function attempts to generate the embeddings for a batch of inputs, most
    // likely of different lengths.
    // The tokenizer expects all inputs in a batch to have the same length, so the
    // following is configuring the tokenizer to pad (add trailing zeros) each input
    // to match the length of the longest in the batch.
    if let Some(pp) = tokenizer.get_padding_mut() {
        pp.strategy = tokenizers::PaddingStrategy::BatchLongest
    } else {
        let pp = PaddingParams {
            strategy: tokenizers::PaddingStrategy::BatchLongest,
            ..Default::default()
        };
        tokenizer.with_padding(Some(pp));
    }
    let tokens = tokenizer
        .encode_batch(data, true)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let token_ids = tokens
        .iter()
        .map(|tokens| {
            let tokens = tokens.get_ids().to_vec();
            Ok(candle::Tensor::new(
                tokens.as_slice(),
                &candle::Device::Cpu,
            )?)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let model = load_model(model_file).await?;

    // Execute the model's forward propagation function, which generates the raw embeddings.
    let token_ids = candle::Tensor::stack(&token_ids, 0)?;
    let embeddings = model.forward(&token_ids, &token_ids.zeros_like()?)?;

    // SBERT adds a pooling operation to the raw output to derive a fixed sized sentence embedding.
    // The BERT models suggest using mean pooling, which is what the operation below performs.
    // https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2#usage-huggingface-transformers
    let (_, n_tokens, _) = embeddings.dims3()?;
    let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
    tracing::debug!("Number of tokens: {}", n_tokens);

    // Take each sentence embedding from the batch and arrange it in the final result tensor.
    // Normalize each embedding as the last step (this generates vectors with length 1, which
    // makes the cosine similarity function significantly more efficient (it becomes a simple
    // dot product).
    let mut results: Vec<Vec<f32>> = Vec::new();
    for j in 0..n_sentences {
        let e_j = embeddings.get(j)?;
        let mut emb: Vec<f32> = e_j.to_vec1()?;
        let length: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        emb.iter_mut().for_each(|x| *x /= length);
        results.push(emb);
    }

    Ok(results)
}

async fn load_model(model_file: &Path) -> anyhow::Result<BertModel> {
    let buffer = tokio::fs::read(model_file)
        .await
        .with_context(|| format!("failed to read model file {model_file:?}"))?;
    let weights = safetensors::SafeTensors::deserialize(&buffer)?;
    let vb = VarBuilder::from_safetensors(vec![weights], DType::F32, &candle::Device::Cpu);
    let model = BertModel::load(vb, &Config::default()).context("error loading bert model")?;
    Ok(model)
}

#[async_trait]
impl wasi_llm::Host for LlmEngine {
    async fn infer(
        &mut self,
        m: wasi_llm::InferencingModel,
        p: String,
        _params: Option<wasi_llm::InferencingParams>,
    ) -> anyhow::Result<Result<wasi_llm::InferencingResult, wasi_llm::Error>> {
        Ok(self.run(m, p).await)
    }

    async fn generate_embeddings(
        &mut self,
        m: wasi_llm::EmbeddingModel,
        data: Vec<String>,
    ) -> anyhow::Result<Result<Vec<Vec<f32>>, wasi_llm::Error>> {
        Ok(self.generate_embeddings(m, data).await)
    }
}

fn model_name(model: wasi_llm::InferencingModel) -> Result<&'static str, wasi_llm::Error> {
    match model {
        wasi_llm::InferencingModel::Llama2V13bChat => Ok("llama2-13b-chat"),
        _ => Err(wasi_llm::Error::ModelNotSupported),
    }
}

fn model_arch(model: &wasi_llm::InferencingModel) -> Result<ModelArchitecture, wasi_llm::Error> {
    match model {
        wasi_llm::InferencingModel::Llama2V70bChat
        | wasi_llm::InferencingModel::Llama2V13bChat
        | wasi_llm::InferencingModel::Llama2V7bChat => Ok(ModelArchitecture::Llama),
        wasi_llm::InferencingModel::Other(_) => Err(wasi_llm::Error::ModelNotSupported),
    }
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
