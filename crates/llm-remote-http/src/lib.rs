use anyhow::Result;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use spin_core::async_trait;
use spin_llm::LlmEngine;
use spin_world::llm::{self as wasi_llm, InferencingParams};
use std::collections::HashSet;

#[derive(Clone)]
pub struct RemoteHttpLlmEngine {
    auth_token: String,
    url: Url,
    client: Option<Client>,
    allowed_models: HashSet<String>,
}

#[derive(Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct InferRequestBodyParams {
    max_tokens: u32,
    repeat_penalty: f32,
    repeat_penalty_last_n_token_count: u32,
    temperature: f32,
    top_k: u32,
    top_p: f32,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
struct InferUsage {
    prompt_token_count: u32,
    generated_token_count: u32,
}
#[derive(Deserialize)]
struct InferResponseBody {
    text: String,
    usage: InferUsage,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
struct EmbeddingUsage {
    prompt_token_count: u32,
}

#[derive(Deserialize)]
struct EmbeddingResponseBody {
    embeddings: Vec<Vec<f32>>,
    usage: EmbeddingUsage,
}

#[async_trait]
impl LlmEngine for RemoteHttpLlmEngine {
    fn add_allowed_models(&mut self, models: HashSet<String>) {
        self.allowed_models = models;
    }

    async fn infer(
        &mut self,
        model: wasi_llm::InferencingModel,
        prompt: String,
        params: InferencingParams,
    ) -> Result<wasi_llm::InferencingResult, wasi_llm::Error> {
        let client = self.client.get_or_insert_with(Default::default);

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("bearer {}", self.auth_token)).unwrap(),
        );
        let inference_options = InferRequestBodyParams {
            max_tokens: params.max_tokens,
            repeat_penalty: params.repeat_penalty,
            repeat_penalty_last_n_token_count: params.repeat_penalty_last_n_token_count,
            temperature: params.temperature,
            top_k: params.top_k,
            top_p: params.top_p,
        };
        let body = serde_json::to_string(&json!({
            "model": model,
            "prompt": prompt,
            "options": inference_options
        }))
        .unwrap();

        let resp = client
            .request(http::Method::POST, self.url.join("/infer").unwrap())
            .headers(headers)
            .body(body)
            .send()
            .await;

        match resp {
            Ok(resp) => match resp.json::<InferResponseBody>().await {
                Ok(val) => Ok(wasi_llm::InferencingResult {
                    text: val.text,
                    usage: wasi_llm::InferencingUsage {
                        prompt_token_count: val.usage.prompt_token_count,
                        generated_token_count: val.usage.generated_token_count,
                    },
                }),
                Err(err) => Err(wasi_llm::Error::RuntimeError(format!(
                    "Failed to deserialize response: {err}"
                ))),
            },
            Err(err) => Err(wasi_llm::Error::RuntimeError(err.to_string())),
        }
    }

    async fn generate_embeddings(
        &mut self,
        model: wasi_llm::EmbeddingModel,
        data: Vec<String>,
    ) -> Result<wasi_llm::EmbeddingsResult, wasi_llm::Error> {
        let client = self.client.get_or_insert_with(Default::default);

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("bearer {}", self.auth_token)).unwrap(),
        );

        let body = serde_json::to_string(&json!({
            "model": model,
            "input": data
        }))
        .unwrap();

        let resp = client
            .request(http::Method::POST, self.url.join("/embed").unwrap())
            .headers(headers)
            .body(body)
            .send()
            .await;

        match resp {
            Ok(resp) => match resp.json::<EmbeddingResponseBody>().await {
                Ok(val) => Ok(wasi_llm::EmbeddingsResult {
                    embeddings: val.embeddings,
                    usage: wasi_llm::EmbeddingsUsage {
                        prompt_token_count: val.usage.prompt_token_count,
                    },
                }),
                Err(err) => Err(wasi_llm::Error::RuntimeError(format!(
                    "Failed to deserialize response: {err}"
                ))),
            },
            Err(err) => Err(wasi_llm::Error::RuntimeError(err.to_string())),
        }
    }
}

impl RemoteHttpLlmEngine {
    pub fn new(url: Url, auth_token: String) -> Self {
        RemoteHttpLlmEngine {
            url,
            auth_token,
            allowed_models: Default::default(),
            client: None,
        }
    }
}
