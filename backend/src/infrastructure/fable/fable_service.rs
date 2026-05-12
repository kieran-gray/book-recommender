use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::warn;
use worker::console_warn;

use crate::{
    application::AppError, domain::BookCandidate, infrastructure::http_client::HttpClientTrait,
};

const FABLE_API_BASE: &str = "https://api.fable.co/api";
const MAX_LIST_PAGES: usize = 25;
const PAGE_SIZE: usize = 100;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FableProfile {
    pub user_id: String,
    pub display_name: String,
    pub lists: Vec<FableBookList>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FableBookList {
    pub id: String,
    pub name: String,
    pub count: u32,
}

#[async_trait(?Send)]
pub trait FableServiceTrait: Send + Sync {
    async fn get_profile(&self, username: &str) -> Result<FableProfile, AppError>;
    async fn get_books_in_list(&self, list_id: &str) -> Result<Vec<BookCandidate>, AppError>;
}

pub struct FableService {
    http: Arc<dyn HttpClientTrait>,
}

impl FableService {
    pub fn create(http: Arc<dyn HttpClientTrait>) -> Arc<Self> {
        Arc::new(Self { http })
    }
}

#[derive(Deserialize)]
struct UsernameResponse {
    id: String,
    #[serde(default)]
    display_name: String,
}

#[derive(Deserialize)]
struct BookListsResponse {
    results: Vec<RawBookList>,
}

#[derive(Deserialize)]
struct RawBookList {
    id: String,
    name: String,
    #[serde(default)]
    count: u32,
}

#[derive(Deserialize)]
struct BooksPage {
    #[serde(default)]
    next: Option<String>,
    results: Vec<BookListEntry>,
}

#[derive(Deserialize)]
struct BookListEntry {
    book: RawBook,
}

#[derive(Deserialize)]
struct RawBook {
    #[serde(default)]
    title: String,
    #[serde(default)]
    subtitle: Option<String>,
    #[serde(default)]
    authors: Vec<RawAuthor>,
    #[serde(default)]
    subjects: Vec<Vec<String>>,
}

#[derive(Deserialize)]
struct RawAuthor {
    #[serde(default)]
    name: String,
}

#[async_trait(?Send)]
impl FableServiceTrait for FableService {
    async fn get_profile(&self, username: &str) -> Result<FableProfile, AppError> {
        let username = sanitize_slug(username)?;
        let user_url = format!("{FABLE_API_BASE}/usernames/{username}");
        let user_value = self
            .http
            .get(&user_url, fable_headers())
            .await
            .map_err(|err| upstream_error("fetch Fable user", &err))?;

        let user: UsernameResponse = serde_json::from_value(user_value).map_err(|err| {
            AppError::InternalError(format!("Failed to parse Fable user response: {err}"))
        })?;

        let lists_url = format!("{FABLE_API_BASE}/v2/users/{}/book_lists/", user.id);
        let lists_value = self
            .http
            .get(&lists_url, fable_headers())
            .await
            .map_err(|err| upstream_error("fetch Fable book lists", &err))?;

        let lists: BookListsResponse = serde_json::from_value(lists_value).map_err(|err| {
            AppError::InternalError(format!("Failed to parse Fable book lists response: {err}"))
        })?;

        let display_name = if user.display_name.is_empty() {
            username
        } else {
            user.display_name
        };

        let lists = lists
            .results
            .into_iter()
            .filter(|list| list.count > 0)
            .map(|list| FableBookList {
                id: list.id,
                name: list.name,
                count: list.count,
            })
            .collect();

        Ok(FableProfile {
            user_id: user.id,
            display_name,
            lists,
        })
    }

    async fn get_books_in_list(&self, list_id: &str) -> Result<Vec<BookCandidate>, AppError> {
        let list_id = sanitize_slug(list_id)?;
        let mut books = Vec::new();
        let mut next_url = Some(format!(
            "{FABLE_API_BASE}/book_lists/{list_id}/books?limit={PAGE_SIZE}"
        ));

        for _ in 0..MAX_LIST_PAGES {
            let Some(url) = next_url.take() else {
                break;
            };
            let value = self
                .http
                .get(&url, fable_headers())
                .await
                .map_err(|err| upstream_error("fetch Fable book list", &err))?;

            let page: BooksPage = serde_json::from_value(value).map_err(|err| {
                AppError::InternalError(format!("Failed to parse Fable book list response: {err}"))
            })?;

            for entry in page.results {
                if let Some(candidate) = to_book_candidate(entry.book) {
                    books.push(candidate);
                }
            }

            next_url = page.next;
        }

        if next_url.is_some() {
            console_warn!(
                "fable: stopped paginating after {} pages; remaining pages skipped",
                MAX_LIST_PAGES
            );
            warn!(
                max_pages = MAX_LIST_PAGES,
                "Fable book list pagination truncated"
            );
        }

        Ok(books)
    }
}

fn to_book_candidate(book: RawBook) -> Option<BookCandidate> {
    let title = book.title.trim();
    if title.is_empty() {
        return None;
    }

    let full_title = match book.subtitle.as_deref().map(str::trim) {
        Some(subtitle) if !subtitle.is_empty() => format!("{title}: {subtitle}"),
        _ => title.to_string(),
    };

    let authors: Vec<String> = book
        .authors
        .into_iter()
        .map(|author| author.name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();
    let author = if authors.is_empty() {
        "Unknown author".to_string()
    } else {
        authors.join(", ")
    };

    let mut shelves: Vec<String> = Vec::new();
    for group in book.subjects {
        for subject in group {
            let trimmed = subject.trim().to_string();
            if trimmed.is_empty() || shelves.contains(&trimmed) {
                continue;
            }
            shelves.push(trimmed);
        }
    }

    Some(BookCandidate {
        title: full_title,
        author,
        average_rating: None,
        shelves,
    })
}

fn fable_headers() -> Vec<(&'static str, &'static str)> {
    vec![("Accept", "application/json")]
}

fn upstream_error(context: &str, err: &str) -> AppError {
    if err.contains("HTTP 404") {
        AppError::ValidationError(format!("Could not {context}: not found"))
    } else {
        AppError::InternalError(format!("Could not {context}: {err}"))
    }
}

fn sanitize_slug(input: &str) -> Result<String, AppError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError(
            "Identifier cannot be empty".to_string(),
        ));
    }
    if trimmed.len() > 128
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::ValidationError(format!(
            "Invalid identifier: {trimmed}"
        )));
    }
    Ok(trimmed.to_string())
}
