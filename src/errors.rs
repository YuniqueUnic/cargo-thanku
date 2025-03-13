use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("HTTP client error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Cargo metadata error: {0}")]
    MetadataError(#[from] cargo_metadata::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<String> for AppError {
    fn from(error: String) -> Self {
        Self::Unknown(error)
    }
}

pub type AppResult<T> = Result<T, AppError>;
