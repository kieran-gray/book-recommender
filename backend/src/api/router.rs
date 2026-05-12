use crate::{
    AppState,
    api::{
        middleware::{create_options_handler, public},
        routes::{
            fable::{fable_book_list_handler, fable_profile_handler},
            recommend_book::recommend_book_handler,
        },
    },
};

use worker::Router;

pub fn create_router(app_state: AppState) -> Router<'static, AppState> {
    Router::with_data(app_state)
        .post_async("/api/v1/books/recommend", |req, ctx| {
            public(recommend_book_handler, req, ctx)
        })
        .options("/api/v1/books/recommend", create_options_handler)
        .get_async("/api/v1/fable/profile/:username", |req, ctx| {
            public(fable_profile_handler, req, ctx)
        })
        .options("/api/v1/fable/profile/:username", create_options_handler)
        .get_async("/api/v1/fable/book_lists/:list_id/books", |req, ctx| {
            public(fable_book_list_handler, req, ctx)
        })
        .options(
            "/api/v1/fable/book_lists/:list_id/books",
            create_options_handler,
        )
}
