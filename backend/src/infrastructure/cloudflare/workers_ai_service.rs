use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use tracing::error;
use worker::{Ai, console_error, console_log};

use crate::application::{AiInferenceServiceTrait, AppError};

pub struct WorkersAiService {
    ai: Ai,
    generation_model: String,
}

impl WorkersAiService {
    pub fn create(ai: Ai, generation_model: String) -> Arc<Self> {
        Arc::new(Self {
            ai,
            generation_model,
        })
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

fn extract_text(raw: &Value) -> Option<String> {
    if let Some(s) = raw.get("response").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    if let Some(s) = raw
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
    {
        return Some(s.to_string());
    }
    if let Some(s) = raw
        .get("result")
        .and_then(|r| r.get("response"))
        .and_then(Value::as_str)
    {
        return Some(s.to_string());
    }
    None
}

#[async_trait(?Send)]
impl AiInferenceServiceTrait for WorkersAiService {
    async fn generate_text(&self, system: &str, user: &str) -> Result<String, AppError> {
        console_log!(
            "book-recommender: starting Workers AI generation model={} prompt_chars={}",
            self.generation_model,
            user.len()
        );
        let request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system,
                },
                ChatMessage {
                    role: "user",
                    content: user,
                },
            ],
            stream: false,
        };

        let raw: Value = self
            .ai
            .run(&self.generation_model, request)
            .await
            .map_err(|e| {
                console_error!("book-recommender: Workers AI generation failed: {}", e);
                error!(error = %e, "Workers AI generation failed");
                AppError::InternalError(format!("Generation failed: {e}"))
            })?;

        let text = extract_text(&raw).ok_or_else(|| {
            console_error!(
                "book-recommender: unexpected Workers AI response shape: {}",
                raw
            );
            AppError::InternalError(format!("Unexpected AI response shape: {raw}"))
        })?;

        console_log!(
            "book-recommender: Workers AI generation completed response_chars={}",
            text.len()
        );
        Ok(text)
    }
}
