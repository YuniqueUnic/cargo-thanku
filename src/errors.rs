use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("GitHub API error: {0}")]
    GitHubOperation(#[source] anyhow::Error),

    #[error("Crates.io API error: {0}")]
    CratesioError(#[from] reqwest::Error),

    #[error("Metadata error: {0}")]
    MetadataError(#[from] cargo_metadata::Error),
}
