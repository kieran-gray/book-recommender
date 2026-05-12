use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{error, info, warn};
use worker::{console_error, console_log, console_warn};

use crate::{
    application::AppError,
    domain::{BookCandidate, BookRecommendation, RecommendationSession},
};

const MAX_BOOKS: usize = 80;
const GENERATE_QUESTIONS_SYSTEM_PROMPT: &str =
    include_str!("prompts/generate_questions_system_prompt.txt");
const RECOMMEND_BOOK_SYSTEM_PROMPT: &str = include_str!("prompts/recommend_book_system_prompt.txt");
const LIBRARY_USER_PROMPT: &str = include_str!("prompts/library_user_prompt.txt");

#[async_trait(?Send)]
pub trait AiInferenceServiceTrait {
    async fn generate_text(&self, system: &str, user: &str) -> Result<String, AppError>;
}

#[async_trait(?Send)]
pub trait BookRecommenderServiceTrait {
    async fn recommend(
        &self,
        session: &RecommendationSession,
    ) -> Result<RecommendationDecision, AppError>;
}

pub struct BookRecommenderService {
    ai: Arc<dyn AiInferenceServiceTrait + Send + Sync>,
}

const TARGET_RECOMMENDATIONS: usize = 3;

#[derive(Debug, PartialEq)]
pub enum RecommendationDecision {
    AskQuestions(Vec<String>),
    Recommend(Vec<BookRecommendation>),
}

#[derive(Deserialize)]
struct AiDecision {
    done: bool,
    questions: Option<Vec<String>>,
    question: Option<String>,
    recommendation: Option<BookRecommendation>,
    recommendations: Option<Vec<BookRecommendation>>,
}

impl BookRecommenderService {
    pub fn create(ai: Arc<dyn AiInferenceServiceTrait + Send + Sync>) -> Arc<Self> {
        Arc::new(Self { ai })
    }
}

#[async_trait(?Send)]
impl BookRecommenderServiceTrait for BookRecommenderService {
    async fn recommend(
        &self,
        session: &RecommendationSession,
    ) -> Result<RecommendationDecision, AppError> {
        session.validate().map_err(AppError::ValidationError)?;

        match self.ask_ai(session).await {
            Ok(decision) => {
                match &decision {
                    RecommendationDecision::AskQuestions(questions) => {
                        console_log!(
                            "book-recommender: AI generated {} questions for {} books",
                            questions.len(),
                            session.books.len()
                        );
                        info!(
                            book_count = session.books.len(),
                            question_count = questions.len(),
                            "Book recommender AI generated questions"
                        );
                    }
                    RecommendationDecision::Recommend(recommendations) => {
                        let titles = recommendations
                            .iter()
                            .map(|r| r.title.as_str())
                            .collect::<Vec<_>>()
                            .join(" | ");
                        console_log!(
                            "book-recommender: AI recommended {} books after {} answers: {}",
                            recommendations.len(),
                            session.answers.len(),
                            titles
                        );
                        info!(
                            book_count = session.books.len(),
                            answer_count = session.answers.len(),
                            recommendation_count = recommendations.len(),
                            titles = %titles,
                            "Book recommender AI generated recommendations"
                        );
                    }
                }
                Ok(decision)
            }
            Err(err) => {
                console_error!(
                    "book-recommender: AI failed in phase={} for books={} answers={}; using fallback; error={:?}",
                    if session.answers.is_empty() {
                        "questions"
                    } else {
                        "recommendation"
                    },
                    session.books.len(),
                    session.answers.len(),
                    err
                );
                error!(
                    error = ?err,
                    book_count = session.books.len(),
                    answer_count = session.answers.len(),
                    phase = if session.answers.is_empty() { "questions" } else { "recommendation" },
                    "Book recommender AI failed; using deterministic fallback"
                );
                Ok(self.fallback_decision(session))
            }
        }
    }
}

impl BookRecommenderService {
    async fn ask_ai(
        &self,
        session: &RecommendationSession,
    ) -> Result<RecommendationDecision, AppError> {
        let system = if session.answers.is_empty() {
            GENERATE_QUESTIONS_SYSTEM_PROMPT
        } else {
            RECOMMEND_BOOK_SYSTEM_PROMPT
        };

        let raw = self
            .ai
            .generate_text(system, &build_prompt(session))
            .await?;
        console_log!(
            "book-recommender: AI call completed phase={} books={} answers={} response_chars={}",
            if session.answers.is_empty() {
                "questions"
            } else {
                "recommendation"
            },
            session.books.len(),
            session.answers.len(),
            raw.len()
        );
        info!(
            book_count = session.books.len(),
            answer_count = session.answers.len(),
            response_chars = raw.len(),
            phase = if session.answers.is_empty() {
                "questions"
            } else {
                "recommendation"
            },
            "Book recommender AI call completed"
        );
        parse_ai_decision(&raw)
    }

    fn fallback_decision(&self, session: &RecommendationSession) -> RecommendationDecision {
        if session.answers.is_empty() {
            return RecommendationDecision::AskQuestions(library_aware_questions(session));
        }

        let mut ranked: Vec<&BookCandidate> = session.books.iter().collect();
        ranked.sort_by(|a, b| {
            b.average_rating
                .unwrap_or(0.0)
                .partial_cmp(&a.average_rating.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if ranked.is_empty() {
            return RecommendationDecision::AskQuestions(vec![
                "I could not find any unread books in the import. Can you check the source?"
                    .to_string(),
            ]);
        }

        let recommendations = ranked
            .into_iter()
            .take(TARGET_RECOMMENDATIONS)
            .enumerate()
            .map(|(index, book)| BookRecommendation {
                title: book.title.clone(),
                author: book.author.clone(),
                reason: match index {
                    0 => "Strongest unread option from the imported library based on the information available.".to_string(),
                    1 => "A solid alternative if the top pick does not feel right today.".to_string(),
                    _ => "Another backup pick from the same library.".to_string(),
                },
            })
            .collect();

        RecommendationDecision::Recommend(recommendations)
    }
}

fn parse_ai_decision(raw: &str) -> Result<RecommendationDecision, AppError> {
    let cleaned = extract_json(raw).ok_or_else(|| {
        console_warn!(
            "book-recommender: AI response did not contain JSON; response_preview={}",
            preview(raw)
        );
        warn!(
            response_preview = %preview(raw),
            "Book recommender AI response did not contain parseable JSON"
        );
        AppError::InternalError("Recommendation response did not contain JSON".to_string())
    })?;

    let decision: AiDecision = serde_json::from_str(&cleaned).map_err(|err| {
        console_warn!(
            "book-recommender: AI JSON parse failed; error={}; response_preview={}; json_preview={}",
            err,
            preview(raw),
            preview(&cleaned)
        );
        warn!(
            error = %err,
            response_preview = %preview(raw),
            json_preview = %preview(&cleaned),
            "Book recommender AI JSON parse failed"
        );
        AppError::InternalError(format!("Recommendation JSON parse failed: {err}"))
    })?;

    if decision.done {
        let mut recommendations = decision.recommendations.unwrap_or_default();
        if let Some(single) = decision.recommendation
            && !recommendations.iter().any(|r| {
                r.title.eq_ignore_ascii_case(&single.title)
                    && r.author.eq_ignore_ascii_case(&single.author)
            }) {
                recommendations.insert(0, single);
            }
        let recommendations = dedupe_recommendations(recommendations);
        if recommendations.is_empty() {
            return Err(AppError::InternalError(
                "AI response was done without any recommendations".to_string(),
            ));
        }
        let recommendations = recommendations
            .into_iter()
            .take(TARGET_RECOMMENDATIONS)
            .collect::<Vec<_>>();
        Ok(RecommendationDecision::Recommend(recommendations))
    } else {
        let mut questions = decision.questions.unwrap_or_default();
        if let Some(question) = decision.question {
            questions.push(question);
        }
        let questions = clean_questions(questions);
        if questions.is_empty() {
            console_warn!(
                "book-recommender: AI response contained no usable questions; response_preview={}",
                preview(raw)
            );
            warn!(
                response_preview = %preview(raw),
                "Book recommender AI response contained no usable questions"
            );
            return Err(AppError::InternalError(
                "AI response had no questions".to_string(),
            ));
        }
        Ok(RecommendationDecision::AskQuestions(questions))
    }
}

fn preview(raw: &str) -> String {
    const LIMIT: usize = 500;
    let mut preview = raw
        .chars()
        .take(LIMIT)
        .collect::<String>()
        .replace('\n', "\\n");
    if raw.chars().count() > LIMIT {
        preview.push_str("...");
    }
    preview
}

fn extract_json(raw: &str) -> Option<String> {
    let trimmed = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }

    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(trimmed[start..=end].to_string())
}

fn dedupe_recommendations(recommendations: Vec<BookRecommendation>) -> Vec<BookRecommendation> {
    let mut seen: Vec<(String, String)> = Vec::new();
    let mut out = Vec::new();
    for rec in recommendations {
        let title = rec.title.trim();
        if title.is_empty() {
            continue;
        }
        let key = (title.to_ascii_lowercase(), rec.author.to_ascii_lowercase());
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        out.push(rec);
    }
    out
}

fn clean_questions(questions: Vec<String>) -> Vec<String> {
    let mut cleaned = Vec::new();
    for question in questions {
        let question = question.trim().trim_matches('"').to_string();
        if question.is_empty() || cleaned.iter().any(|q| q == &question) {
            continue;
        }
        cleaned.push(question);
    }
    cleaned
}

fn build_prompt(session: &RecommendationSession) -> String {
    let books = session
        .books
        .iter()
        .take(MAX_BOOKS)
        .map(format_book)
        .collect::<Vec<_>>()
        .join("\n");

    let answers = if session.answers.is_empty() {
        "No answers yet.".to_string()
    } else {
        session
            .answers
            .iter()
            .map(|a| format!("Q: {}\nA: {}", a.question, a.answer))
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    let instruction = if session.answers.is_empty() {
        "Generate the narrowing questions now."
    } else {
        "Recommend now using the answers."
    };

    LIBRARY_USER_PROMPT
        .replace("{{instruction}}", instruction)
        .replace("{{books}}", &books)
        .replace("{{answers}}", &answers)
}

fn format_book(book: &BookCandidate) -> String {
    let rating = book
        .average_rating
        .map(|r| format!("{r:.2}"))
        .unwrap_or_else(|| "unknown".to_string());
    let shelves = if book.shelves.is_empty() {
        "none".to_string()
    } else {
        book.shelves.join(", ")
    };
    format!(
        "- \"{}\" by {}. Goodreads average: {}. Shelves: {}",
        book.title, book.author, rating, shelves
    )
}

fn library_aware_questions(session: &RecommendationSession) -> Vec<String> {
    let top_shelves = top_shelves(session);
    let top_authors = top_authors(session);
    let examples = session
        .books
        .iter()
        .take(12)
        .map(|book| book.title.as_str())
        .collect::<Vec<_>>();

    let genre_question = if top_shelves.len() >= 2 {
        format!(
            "Which of these shelves sounds most appealing right now: {}, or something else?",
            top_shelves.join(", ")
        )
    } else {
        "What kind of story or subject are you most drawn to right now?".to_string()
    };

    let author_question = if top_authors.len() >= 2 {
        format!(
            "Do you want to return to a familiar author like {}, or try someone different?",
            top_authors.join(" or ")
        )
    } else {
        "Would you rather read an author you already know, or take a chance on someone new?"
            .to_string()
    };

    let example_question = if examples.len() >= 3 {
        format!(
            "From this import, which title is closest to today's mood: {}, {}, {}, or none of those?",
            examples[0], examples[1], examples[2]
        )
    } else {
        "Is there any specific unread title you are already half-tempted by?".to_string()
    };

    vec![
        genre_question,
        "How much effort do you want: quick and easy, medium, or something that needs concentration?"
            .to_string(),
        "Should the pick feel light, emotionally rich, tense, strange, or informative?".to_string(),
        author_question,
        example_question,
    ]
}

fn top_shelves(session: &RecommendationSession) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for shelf in session.books.iter().flat_map(|book| book.shelves.iter()) {
        let shelf = shelf.trim();
        if shelf.is_empty() || shelf.eq_ignore_ascii_case("to-read") {
            continue;
        }
        *counts.entry(shelf.to_string()).or_default() += 1;
    }
    top_counts(counts, 4)
}

fn top_authors(session: &RecommendationSession) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for book in &session.books {
        if book.author.trim().is_empty() || book.author == "Unknown author" {
            continue;
        }
        *counts.entry(book.author.clone()).or_default() += 1;
    }
    top_counts(counts, 2)
}

fn top_counts(counts: BTreeMap<String, usize>, limit: usize) -> Vec<String> {
    let mut counts = counts.into_iter().collect::<Vec<_>>();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
        .into_iter()
        .take(limit)
        .map(|(label, _)| label)
        .collect()
}
