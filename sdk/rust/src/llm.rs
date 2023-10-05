use crate::wit::fermyon::spin::llm;

pub use crate::wit::fermyon::spin::llm::{
    EmbeddingsResult, EmbeddingsUsage, Error, InferencingParams, InferencingResult,
    InferencingUsage,
};

/// The model use for inferencing
#[allow(missing_docs)]
pub enum InferencingModel<'a> {
    Llama2Chat,
    CodellamaInstruct,
    Other(&'a str),
}

impl<'a> InferencingModel<'a> {
    fn as_str(&self) -> llm::InferencingModel<'a> {
        match self {
            InferencingModel::Llama2Chat => "llama2-chat",
            InferencingModel::CodellamaInstruct => "codellama-instruct",
            InferencingModel::Other(s) => s,
        }
    }
}

impl Default for InferencingParams {
    fn default() -> Self {
        Self {
            max_tokens: 100,
            repeat_penalty: 1.1,
            repeat_penalty_last_n_token_count: 64,
            temperature: 0.8,
            top_k: 40,
            top_p: 0.9,
        }
    }
}

/// Perform inferencing using the provided model and prompt
pub fn infer(model: InferencingModel, prompt: &str) -> Result<InferencingResult, Error> {
    llm::infer(&model.as_str(), prompt, None)
}

/// Perform inferencing using the provided model, prompt, and options
pub fn infer_with_options(
    model: InferencingModel,
    prompt: &str,
    options: InferencingParams,
) -> Result<InferencingResult, Error> {
    llm::infer(&model.as_str(), prompt, Some(options))
}

/// Model used for generating embeddings
#[allow(missing_docs)]
pub enum EmbeddingModel<'a> {
    AllMiniLmL6V2,
    Other(&'a str),
}

impl<'a> EmbeddingModel<'a> {
    fn as_str(&self) -> llm::EmbeddingModel<'a> {
        match self {
            EmbeddingModel::AllMiniLmL6V2 => "all-minilm-l6-v2",
            EmbeddingModel::Other(s) => s,
        }
    }
}

/// Generate embeddings using the provided model and collection of text
pub fn generate_embeddings(
    model: EmbeddingModel,
    text: &[&str],
) -> Result<llm::EmbeddingsResult, Error> {
    llm::generate_embeddings(&model.as_str(), text)
}
