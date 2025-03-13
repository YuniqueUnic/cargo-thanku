use anyhow::Result;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub enum OutputFormat {
    MarkdownTable,
    MarkdownList,
    Json,
    Toml,
    Yaml,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::MarkdownTable
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
            .ok_or_else(|| anyhow::anyhow!("Global config not initialized"))
    }

    pub fn init(config: Config) -> Result<()> {
        GLOBAL_CONFIG
            .set(config)
            .map_err(|_| anyhow::anyhow!("Global config already initialized"))
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
            .map(|t| match t.as_str() {
                "markdown-table" => OutputFormat::MarkdownTable,
                "markdown-list" => OutputFormat::MarkdownList,
                "json" => OutputFormat::Json,
                "toml" => OutputFormat::Toml,
                "yaml" => OutputFormat::Yaml,
                _ => OutputFormat::default(),
            })
            .unwrap_or_default();

        let link_source = matches
            .get_one::<String>("link")
            .map(|l| match l.as_str() {
                "github" => LinkSource::GitHub,
                "crates-io" => LinkSource::CratesIo,
                "link-empty" => LinkSource::LinkEmpty,
                _ => LinkSource::Other,
            })
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
