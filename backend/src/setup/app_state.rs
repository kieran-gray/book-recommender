use std::sync::Arc;

use worker::{Env, console_log};

use crate::{
    application::{AiInferenceServiceTrait, BookRecommenderService, BookRecommenderServiceTrait},
    infrastructure::{
        FableService, FableServiceTrait, HttpClientTrait, WorkerHttpClient, WorkersAiService,
    },
    setup::{Config, config::InferenceConfig, exceptions::SetupError},
};

#[cfg(feature = "ollama")]
use crate::infrastructure::OllamaInferenceService;

pub struct AppState {
    pub config: Config,
    pub book_recommender_service: Arc<dyn BookRecommenderServiceTrait + Send + Sync>,
    pub fable_service: Arc<dyn FableServiceTrait + Send + Sync>,
}

impl AppState {
    pub fn from_env(env: &Env, config: Config) -> Result<Self, SetupError> {
        let http_client: Arc<dyn HttpClientTrait> = Arc::new(WorkerHttpClient::new());
        let ai_service = create_inference_service(env, &config.ai.inference, http_client.clone())?;
        let book_recommender_service = BookRecommenderService::create(ai_service);
        let fable_service = FableService::create(http_client);

        Ok(Self {
            config,
            book_recommender_service,
            fable_service,
        })
    }
}

fn create_inference_service(
    env: &Env,
    config: &InferenceConfig,
    _http_client: Arc<dyn HttpClientTrait>,
) -> Result<Arc<dyn AiInferenceServiceTrait + Send + Sync>, SetupError> {
    match config {
        InferenceConfig::Cloudflare { generation_model } => {
            console_log!(
                "book-recommender: using Workers AI provider with model={}",
                generation_model
            );
            let ai_binding = env
                .ai("AI")
                .map_err(|_| SetupError::MissingVariable("AI".to_string()))?;
            Ok(WorkersAiService::create(
                ai_binding,
                generation_model.clone(),
            ))
        }

        #[cfg(feature = "ollama")]
        InferenceConfig::Ollama {
            url,
            generation_model,
        } => {
            console_log!(
                "book-recommender: using Ollama provider with model={} url={}",
                generation_model,
                url
            );
            Ok(OllamaInferenceService::create(
                _http_client,
                generation_model.clone(),
                url.clone(),
            ))
        }
    }
}
