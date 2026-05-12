use tracing::error;
use worker::{Request, Response, RouteContext};

use crate::{
    AppState,
    api::schema::{requests::RecommendBookRequest, responses::RecommendBookResponse},
    application::{AppError, RecommendationDecision},
};

pub async fn recommend_book_handler(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> worker::Result<Response> {
    let payload: RecommendBookRequest = match req.json().await {
        Ok(p) => p,
        Err(e) => {
            error!(error = ?e, "Failed to parse book recommender request");
            return Ok(Response::from(AppError::ValidationError(
                "Failed to parse request body".into(),
            )));
        }
    };

    let decision = match ctx.data.book_recommender_service.recommend(&payload).await {
        Ok(decision) => decision,
        Err(e) => return Ok(Response::from(e)),
    };

    let response = match decision {
        RecommendationDecision::AskQuestions(questions) => RecommendBookResponse {
            done: false,
            questions,
            recommendations: Vec::new(),
        },
        RecommendationDecision::Recommend(recommendations) => RecommendBookResponse {
            done: true,
            questions: Vec::new(),
            recommendations,
        },
    };

    Response::from_json(&response)
}
