use anyhow::Result;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::instrument;

use crate::errors::AppError;
use crate::output::OutputFormat;

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
            "other" => Self::Other,
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
    pub crates_token: Option<String>,
    pub language: String,
    pub verbose: bool,
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
            crates_token: None,
            language: String::from("zh"),
            verbose: false,
        }
    }
}

static GLOBAL_CONFIG: OnceLock<Config> = OnceLock::new();

impl Config {
    pub fn global() -> Result<&'static Config> {
        GLOBAL_CONFIG
            .get()
            .ok_or_else(|| anyhow::anyhow!(t!("config.failed_to_initialize_global_config")))
    }

    #[instrument]
    pub fn init(config: Config) -> Result<()> {
        GLOBAL_CONFIG
            .set(config)
            .map_err(|_| anyhow::anyhow!(t!("config.global_config_already_initialized")))
    }

    #[instrument(skip_all)]
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
            .get_one::<String>("format")
            .map(|t| t.parse::<OutputFormat>().unwrap_or_default())
            .unwrap_or_default();

        let link_source = matches
            .get_one::<String>("source")
            .map(|l| l.parse::<LinkSource>().unwrap_or_default())
            .unwrap_or_default();

        let github_token = matches.get_one::<String>("token").cloned();
        let crates_token = matches.get_one::<String>("crates-token").cloned();

        let language = matches
            .get_one::<String>("language")
            .cloned()
            .unwrap_or_default();

        let verbose = matches.get_flag("verbose");

        Ok(Self {
            input,
            output,
            name,
            format,
            link_source,
            github_token,
            crates_token,
            language,
            verbose,
        })
    }

    pub fn get_cargo_toml_path(&self) -> Result<PathBuf> {
        if self.input.is_dir() {
            let path = self.input.join("Cargo.toml");
            if path.exists() {
                return Ok(path);
            }
        }

        if self.input.is_file() && self.input.extension().unwrap_or_default() == "toml" {
            return Ok(self.input.clone());
        }

        anyhow::bail!(t!(
            "config.cargo_toml_not_found",
            path = self.input.display()
        ));
    }
}
