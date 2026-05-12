use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use worker::{console_error, console_log};

use crate::{
    application::{AiInferenceServiceTrait, AppError},
    infrastructure::HttpClientTrait,
};

pub struct OllamaInferenceService {
    client: Arc<dyn HttpClientTrait>,
    generation_model: String,
    ollama_url: String,
}

impl OllamaInferenceService {
    pub fn create(
        client: Arc<dyn HttpClientTrait>,
        generation_model: String,
        ollama_url: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            client,
            generation_model,
            ollama_url: ollama_url.trim_end_matches('/').to_string(),
        })
    }
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    system: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[async_trait(?Send)]
impl AiInferenceServiceTrait for OllamaInferenceService {
    async fn generate_text(&self, system: &str, user: &str) -> Result<String, AppError> {
        let url = format!("{}/api/generate", self.ollama_url);
        console_log!(
            "book-recommender: starting Ollama generation model={} url={} prompt_chars={}",
            self.generation_model,
            url,
            user.len()
        );
        let request = GenerateRequest {
            model: &self.generation_model,
            prompt: user,
            system,
            stream: false,
        };
        let body = serde_json::to_value(request).map_err(|e| {
            AppError::InternalError(format!("Failed to convert ollama request to Value: {e}"))
        })?;

        let response = self
            .client
            .post(&url, body, vec![("Content-Type", "application/json")])
            .await
            .map_err(|e| {
                console_error!("book-recommender: Ollama HTTP request failed: {}", e);
                AppError::InternalError(format!("Ollama client failure: {e}"))
            })?;
        let data: GenerateResponse = serde_json::from_value(response).map_err(|e| {
            console_error!("book-recommender: Ollama response parse failed: {}", e);
            AppError::InternalError(format!("Failed to parse Ollama response: {e}"))
        })?;
        console_log!(
            "book-recommender: Ollama generation completed response_chars={}",
            data.response.len()
        );
        Ok(data.response)
    }
}
