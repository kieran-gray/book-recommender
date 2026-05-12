#[derive(Debug)]
pub enum AppError {
    InternalError(String),
    ValidationError(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::InternalError(msg) => write!(f, "Internal server error: {msg}"),
            AppError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}
