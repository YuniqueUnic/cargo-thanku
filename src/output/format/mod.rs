use std::{io::Read, str::FromStr};

use anyhow::Result;

use crate::{errors::AppError, output::dependency::DependencyInfo};

pub(crate) mod csv;
pub mod markdown;
pub(crate) mod structured;

pub use csv::CsvFormatter;
pub use markdown::{MarkdownListFormatter, MarkdownTableFormatter};
pub use structured::{JsonFormatter, TomlFormatter, YamlFormatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    MarkdownTable,
    MarkdownList,
    Csv,
    Json,
    Yaml,
    Toml,
}

impl OutputFormat {
    pub fn to_identifier(&self) -> &str {
        match self {
            Self::MarkdownTable => "mt",
            Self::MarkdownList => "ml",
            _ => "",
        }
    }

    pub fn to_extension(&self) -> &str {
        match self {
            Self::MarkdownTable | Self::MarkdownList => "md",
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::MarkdownTable
    }
}

impl FromStr for OutputFormat {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "mt" | "markdown-table" => Self::MarkdownTable,
            "ml" | "markdown-list" => Self::MarkdownList,
            "csv" => Self::Csv,
            "json" => Self::Json,
            "toml" => Self::Toml,
            "yml" | "yaml" => Self::Yaml,
            _ => return Err(AppError::InvalidOutputFormat(s.to_string())),
        })
    }
}

pub trait Formatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String>;
    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>>;

    fn parse_reader(&self, reader: &mut dyn Read) -> Result<Vec<DependencyInfo>> {
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer)?;
        self.parse(&buffer)
    }
}

impl dyn Formatter {
    pub fn new(format: OutputFormat) -> Result<Box<dyn Formatter>> {
        Ok(match format {
            OutputFormat::MarkdownTable => Box::new(MarkdownTableFormatter),
            OutputFormat::MarkdownList => Box::new(MarkdownListFormatter),
            OutputFormat::Csv => Box::new(CsvFormatter),
            OutputFormat::Json => Box::new(JsonFormatter),
            OutputFormat::Toml => Box::new(TomlFormatter),
            OutputFormat::Yaml => Box::new(YamlFormatter),
        })
    }
}
