pub mod cloudflare;
pub mod fable;
pub mod http_client;

#[cfg(feature = "ollama")]
pub mod ollama;

pub use cloudflare::workers_ai_service::WorkersAiService;
pub use fable::{FableBookList, FableProfile, FableService, FableServiceTrait};
pub use http_client::{HttpClientTrait, WorkerHttpClient};

#[cfg(feature = "ollama")]
pub use ollama::inference_service::OllamaInferenceService;
