use serde::Serialize;
use serde_json::json;

use crate::{application::AppError, domain::BookRecommendation};

const HTTP_STATUS_BAD_REQUEST: u16 = 400;
const HTTP_STATUS_INTERNAL_ERROR: u16 = 500;

#[derive(PartialEq, Debug, Serialize)]
pub struct RecommendBookResponse {
    pub done: bool,
    pub questions: Vec<String>,
    pub recommendation: Option<BookRecommendation>,
}

impl From<AppError> for worker::Response {
    fn from(error: AppError) -> Self {
        let (status, message) = match &error {
            AppError::InternalError(_) => (HTTP_STATUS_INTERNAL_ERROR, "Internal server error"),
            AppError::ValidationError(msg) => (HTTP_STATUS_BAD_REQUEST, msg.as_str()),
        };

        match worker::Response::from_json(&json!({ "success": false, "error": message })) {
            Ok(response) => response.with_status(status),
            Err(_) => worker::Response::builder()
                .with_status(status)
                .fixed(format!(r#"{{"success":false,"error":"{message}"}}"#).into_bytes()),
        }
    }
}
