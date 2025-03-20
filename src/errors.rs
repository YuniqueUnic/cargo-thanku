use thiserror::Error;

#[allow(clippy::enum_variant_names, dead_code)]
#[derive(Error, Debug)]
pub enum AppError {
    #[error("HTTP client error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Cargo metadata error: {0}")]
    MetadataError(#[from] cargo_metadata::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("Invalid output format: {0}")]
    InvalidOutputFormat(String),

    #[error("Invalid link source: {0}")]
    InvalidLinkSource(String),

    #[error("Unknown error: {0}")]
    Unknown(String),

    #[error("Invalid CSV content: {0}")]
    InvalidCsvContent(String),

    #[error("Invalid dependency kind: {0}")]
    InvalidDependencyKind(String),

    #[error("Invalid source link: {0}")]
    InvalidSourceLink(String),

    #[error("Invalid status: {0}")]
    InvalidStatus(String),

    #[error("Invalid stats: {0}")]
    InvalidStats(String),

    #[error("Invalid table header: {0}")]
    InvalidTableHeader(String),

    #[error("Invalid list line: {0}")]
    InvalidListLine(String),

    #[error("Invalid table line: {0}")]
    InvalidTableLine(String),
}

impl From<String> for AppError {
    fn from(error: String) -> Self {
        Self::Unknown(error)
    }
}
