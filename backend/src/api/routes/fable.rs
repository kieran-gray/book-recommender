use serde::Serialize;
use tracing::error;
use worker::{Request, Response, RouteContext};

use crate::{
    AppState,
    application::AppError,
    domain::BookCandidate,
    infrastructure::{FableBookList, FableProfile},
};

#[derive(Serialize)]
struct ProfileResponse {
    user_id: String,
    display_name: String,
    lists: Vec<FableBookList>,
}

impl From<FableProfile> for ProfileResponse {
    fn from(profile: FableProfile) -> Self {
        Self {
            user_id: profile.user_id,
            display_name: profile.display_name,
            lists: profile.lists,
        }
    }
}

#[derive(Serialize)]
struct BookListBooksResponse {
    books: Vec<BookCandidate>,
}

pub async fn fable_profile_handler(
    _req: Request,
    ctx: RouteContext<AppState>,
) -> worker::Result<Response> {
    let Some(username) = ctx.param("username") else {
        return Ok(Response::from(AppError::ValidationError(
            "Missing username".to_string(),
        )));
    };

    match ctx.data.fable_service.get_profile(username).await {
        Ok(profile) => Response::from_json(&ProfileResponse::from(profile)),
        Err(err) => {
            error!(error = ?err, "Fable profile lookup failed");
            Ok(Response::from(err))
        }
    }
}

pub async fn fable_book_list_handler(
    _req: Request,
    ctx: RouteContext<AppState>,
) -> worker::Result<Response> {
    let Some(list_id) = ctx.param("list_id") else {
        return Ok(Response::from(AppError::ValidationError(
            "Missing list id".to_string(),
        )));
    };

    match ctx.data.fable_service.get_books_in_list(list_id).await {
        Ok(books) => Response::from_json(&BookListBooksResponse { books }),
        Err(err) => {
            error!(error = ?err, "Fable book list lookup failed");
            Ok(Response::from(err))
        }
    }
}
