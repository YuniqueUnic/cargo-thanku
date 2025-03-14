use anyhow::Result;
use std::fmt;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::errors::AppError;

#[derive(Debug, Clone)]
pub enum OutputFormat {
    MarkdownTable,
    MarkdownList,
    Json,
    Toml,
    Yaml,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::MarkdownTable
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "markdown-table" => Self::MarkdownTable,
            "markdown-list" => Self::MarkdownList,
            "json" => Self::Json,
            "toml" => Self::Toml,
            "yaml" => Self::Yaml,
            _ => return Err(AppError::InvalidOutputFormat(s.to_string())),
        })
    }
}

#[derive(Debug, Clone)]
pub enum LinkSource {
    GitHub,
    CratesIo,
    LinkEmpty,
    Other,
}

impl Default for LinkSource {
    fn default() -> Self {
        Self::GitHub
    }
}

impl std::str::FromStr for LinkSource {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "github" => Self::GitHub,
            "crates-io" => Self::CratesIo,
            "link-empty" => Self::LinkEmpty,
            _ => return Err(AppError::InvalidLinkSource(s.to_string())),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub input: PathBuf,
    pub output: PathBuf,
    pub name: String,
    pub format: OutputFormat,
    pub link_source: LinkSource,
    pub github_token: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input: PathBuf::from("Cargo.toml"),
            output: PathBuf::from("thanks.md"),
            name: String::from("thanks"),
            format: OutputFormat::default(),
            link_source: LinkSource::default(),
            github_token: None,
        }
    }
}

static GLOBAL_CONFIG: OnceLock<Config> = OnceLock::new();

impl Config {
    pub fn global() -> Result<&'static Config> {
        GLOBAL_CONFIG
            .get()
            .ok_or_else(|| anyhow::anyhow!(t!("config.failed_to_initialize_global_config.zh")))
    }

    pub fn init(config: Config) -> Result<()> {
        GLOBAL_CONFIG
            .set(config)
            .map_err(|_| anyhow::anyhow!(t!("config.global_config_already_initialized.zh")))
    }

    pub fn from_matches(matches: &clap::ArgMatches) -> Result<Self> {
        let input = matches
            .get_one::<PathBuf>("input")
            .cloned()
            .unwrap_or_else(|| PathBuf::from("Cargo.toml"));

        let output = matches
            .get_one::<PathBuf>("output")
            .cloned()
            .unwrap_or_else(|| PathBuf::from("thanks.md"));

        let name = matches
            .get_one::<String>("name")
            .cloned()
            .unwrap_or_else(|| String::from("thanks"));

        let format = matches
            .get_one::<String>("type")
            .map(|t| t.parse::<OutputFormat>().unwrap_or_default())
            .unwrap_or_default();

        let link_source = matches
            .get_one::<String>("link")
            .map(|l| l.parse::<LinkSource>().unwrap_or_default())
            .unwrap_or_default();

        let github_token = matches.get_one::<String>("token").cloned();

        Ok(Self {
            input,
            output,
            name,
            format,
            link_source,
            github_token,
        })
    }
}
