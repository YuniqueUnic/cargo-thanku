use std::path::PathBuf;

use crate::cli;

#[derive(Debug)]
pub struct Config {
    pub input: PathBuf,
    pub output_file: PathBuf,
    pub output_format: OutputFormat,
    pub token: String,
}

#[derive(Debug)]
pub enum OutputFormat {
    MarkdownTable,
    Json,
    Toml,
    Yaml,
}

impl Config {
    pub fn global() -> anyhow::Result<Self> {
        let matches = cli::build_cli().get_matches();

        let input = matches
            .get_one::<String>("input")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("Cargo.toml"));
        let output_file = matches
            .get_one::<String>("output")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("thanks.md"));
        let output_format = match matches.get_one::<String>("type").map(String::as_str) {
            Some("json") => OutputFormat::Json,
            Some("toml") => OutputFormat::Toml,
            Some("yaml") => OutputFormat::Yaml,
            _ => OutputFormat::MarkdownTable,
        };
        let token = matches
            .get_one::<String>("token")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("GitHub token is required"))?;

        Ok(Self {
            input,
            output_file,
            output_format,
            token,
        })
    }
}
