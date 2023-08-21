use crate::wit::fermyon::spin::llm;

pub use crate::wit::fermyon::spin::llm::{
    generate_embeddings, EmbeddingModel, Error, InferencingModel, InferencingParams,
    InferencingResult,
};

/// Perform inferencing using the provided model and prompt
pub fn infer(model: InferencingModel, prompt: &str) -> Result<InferencingResult, Error> {
    llm::infer(model, prompt, None)
}

/// Perform inferencing using the provided model, prompt, and options
pub fn infer_with_options(
    model: InferencingModel,
    prompt: &str,
    options: InferencingParams,
) -> Result<InferencingResult, Error> {
    llm::infer(model, prompt, Some(options))
}
