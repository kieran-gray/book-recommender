use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BookCandidate {
    pub title: String,
    pub author: String,
    pub average_rating: Option<f32>,
    pub shelves: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BookAnswer {
    pub question: String,
    pub answer: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BookRecommendation {
    pub title: String,
    pub author: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RecommendationSession {
    pub books: Vec<BookCandidate>,
    pub answers: Vec<BookAnswer>,
}

impl RecommendationSession {
    pub fn validate(&self) -> Result<(), String> {
        if self.books.is_empty() {
            return Err("No unread books were provided".to_string());
        }

        if self.books.iter().any(|book| book.title.trim().is_empty()) {
            return Err("Every book must have a title".to_string());
        }

        Ok(())
    }
}
