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

/// 输出目标枚举
pub enum OutputWriter {
    Stdout(std::io::Stdout),
    File(std::fs::File),
}

impl std::io::Write for OutputWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Stdout(stdout) => stdout.write(buf),
            Self::File(file) => file.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Stdout(stdout) => stdout.flush(),
            Self::File(file) => file.flush(),
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct Config {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub format: OutputFormat,
    pub link_source: LinkSource,
    pub github_token: Option<String>,
    // pub crates_token: Option<String>,
    pub no_relative_libs: bool,
    pub language: String,
    pub verbose: bool,
    pub max_concurrent_requests: usize,
    pub max_retries: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input: PathBuf::from("Cargo.toml"),
            output: None,
            format: OutputFormat::default(),
            link_source: LinkSource::default(),
            github_token: None,
            // crates_token: None,
            no_relative_libs: false,
            language: String::from("zh"),
            verbose: false,
            max_concurrent_requests: 5,
            max_retries: 3,
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

        let output = matches.get_one::<PathBuf>("output").cloned();

        let format = matches
            .get_one::<String>("format")
            .map(|f| f.parse::<OutputFormat>().unwrap_or_default())
            .unwrap_or_default();

        let link_source = matches
            .get_one::<String>("source")
            .map(|l| l.parse::<LinkSource>().unwrap_or_default())
            .unwrap_or_default();

        let github_token = matches.get_one::<String>("token").cloned();
        // let crates_token = matches.get_one::<String>("crates-token").cloned();
        let no_relative_libs = matches.get_flag("no-relative-libs");

        let language = matches
            .get_one::<String>("language")
            .cloned()
            .unwrap_or_default();

        let verbose = matches.get_flag("verbose");

        let max_concurrent_requests = matches.get_one::<usize>("concurrent").copied().unwrap_or(5);

        let max_retries = matches.get_one::<u32>("retries").copied().unwrap_or(3);

        Ok(Self {
            input,
            output,
            format,
            link_source,
            github_token,
            // crates_token,
            no_relative_libs,
            language,
            verbose,
            max_concurrent_requests,
            max_retries,
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

    /// 获取输出位置 (buffer)
    ///
    /// - 如果输出位置是文件，则返回文件内容进行追加写入
    ///     - 如果文件不存在，则创建，然后返回文件内容进行写入
    /// - 如果输出位置是标准输出，则返回标准输出，进行写入
    pub fn get_output_writer(&self) -> Result<OutputWriter> {
        match &self.output {
            Some(path) if path.as_os_str() == "-" => Ok(OutputWriter::Stdout(std::io::stdout())),
            Some(path) => {
                if path.exists() {
                    // 文件存在，则打开文件进行追加写入
                    let file = std::fs::OpenOptions::new()
                        .write(true)
                        .append(true)
                        .open(path)
                        .map_err(|e| {
                            anyhow::anyhow!(t!(
                                "config.failed_to_open_output_file",
                                path = path.display(),
                                error = e.to_string()
                            ))
                        })?;
                    Ok(OutputWriter::File(file))
                } else {
                    // 文件不存在，则创建文件并返回文件内容进行写入
                    let file = std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(path)
                        .map_err(|e| {
                            anyhow::anyhow!(t!(
                                "config.failed_to_open_output_file",
                                path = path.display(),
                                error = e.to_string()
                            ))
                        })?;
                    Ok(OutputWriter::File(file))
                }
            }
            None => Ok(OutputWriter::Stdout(std::io::stdout())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_writer_stdout() -> Result<()> {
        let mut config = Config::default();
        config.output = Some(PathBuf::from("-"));

        match config.get_output_writer()? {
            OutputWriter::Stdout(_) => Ok(()),
            _ => panic!("Expected Stdout writer"),
        }
    }

    #[test]
    fn test_output_writer_file() -> Result<()> {
        let temp_file = assert_fs::NamedTempFile::new("test-output.md")?;
        let mut config = Config::default();
        config.output = Some(temp_file.path().to_path_buf());

        match config.get_output_writer()? {
            OutputWriter::File(_) => Ok(()),
            _ => panic!("Expected File writer"),
        }
    }

    #[test]
    fn test_output_writer_default() -> Result<()> {
        let config = Config::default();

        match config.get_output_writer()? {
            OutputWriter::Stdout(_) => Ok(()),
            _ => panic!("Expected default Stdout writer"),
        }
    }
}
