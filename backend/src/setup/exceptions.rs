#[derive(Debug)]
pub enum SetupError {
    MissingVariable(String),
    InvalidVariable(String),
    MissingBinding(String),
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupError::MissingVariable(msg) => write!(f, "Environment Variable Not Found: {msg}"),
            SetupError::InvalidVariable(msg) => write!(f, "Invalid Environment Variable: {msg}"),
            SetupError::MissingBinding(msg) => write!(f, "Binding Not Found: {msg}"),
        }
    }
}

impl std::error::Error for SetupError {}
