use anyhow::Result;
use rust_i18n::t;
use serde::Serialize;
use std::io::{self, Write};

use crate::{errors::AppError, sources::Source};

/// 定义输出格式
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    MarkdownTable,
    MarkdownList,
    Json,
    Toml,
    Yaml,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

/// 表示一个依赖项的信息
#[derive(Debug, Serialize, Clone)]
pub struct DependencyInfo {
    pub name: String,
    pub description: Option<String>,
    pub source_type: String,
    pub source_url: Option<String>,
    pub stats: DependencyStats,
    pub failed: bool,
    pub error_message: Option<String>,
}

/// 依赖项的统计信息
#[derive(Debug, Serialize, Clone)]
pub struct DependencyStats {
    pub stars: Option<u32>,
    pub downloads: Option<u32>,
}

/// 格式化器特征
pub trait Formatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String>;
}

/// Markdown 表格格式化器
pub struct MarkdownTableFormatter;

impl Formatter for MarkdownTableFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let mut output = String::new();

        // 表头
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            t!("output.name"),
            t!("output.description"),
            t!("output.source"),
            t!("output.stats"),
            t!("output.status")
        ));
        output.push_str("|------|--------|--------|-------|--------|\n");

        // 内容
        for dep in deps {
            let description = match dep.description {
                Some(ref description) => description.replace("\n", " "),
                None => "unknown".to_string(),
            };

            let stats = match (dep.stats.stars, dep.stats.downloads) {
                (Some(stars), _) => format!("🌟 {}", stars),
                (None, Some(downloads)) => format!("📦 {}", downloads),
                _ => "❓".to_string(),
            };

            let source = if let Some(url) = &dep.source_url {
                format!("[{}]({})", dep.source_type, url)
            } else {
                dep.source_type.clone()
            };

            let status = if dep.failed {
                format!("❌ {}", dep.error_message.as_deref().unwrap_or("Failed"))
            } else {
                "✅".to_string()
            };

            output.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                dep.name, description, source, stats, status
            ));
        }

        Ok(output)
    }
}

/// Markdown 列表格式化器
pub struct MarkdownListFormatter;

impl Formatter for MarkdownListFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let mut output = String::new();
        output.push_str(&format!("# {}\n\n", t!("output.dependencies")));

        for dep in deps {
            let description = match dep.description {
                Some(ref description) => description.replace("\n", " "), // 将 description 多行变为一行
                None => "unknown".to_string(),
            };

            let stats = match (dep.stats.stars, dep.stats.downloads) {
                (Some(stars), _) => format!("🌟 {}", stars),
                (None, Some(downloads)) => format!("📦 {}", downloads),
                _ => "❓".to_string(),
            };

            let status = if dep.failed {
                format!("❌ {}", dep.error_message.as_deref().unwrap_or("Failed"))
            } else {
                "✅".to_string()
            };

            if let Some(url) = &dep.source_url {
                output.push_str(&format!(
                    "- {} [{}]({}) ({}) {}\n",
                    dep.name, description, url, stats, status
                ));
            } else {
                output.push_str(&format!(
                    "- {} [{}] ({}) {}\n",
                    dep.name, description, stats, status
                ));
            }
        }

        Ok(output)
    }
}

/// JSON 格式化器
pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        Ok(serde_json::to_string_pretty(deps)?)
    }
}

/// TOML 格式化器
pub struct TomlFormatter;

impl Formatter for TomlFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        Ok(toml::to_string_pretty(deps)?)
    }
}

/// YAML 格式化器
pub struct YamlFormatter;

impl Formatter for YamlFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        Ok(serde_yaml::to_string(deps)?)
    }
}

// 输出管理器
pub struct OutputManager<W: Write> {
    formatter: Box<dyn Formatter>,
    writer: W,
}

impl<W: Write> OutputManager<W> {
    pub fn new(format: OutputFormat, writer: W) -> Self {
        let formatter: Box<dyn Formatter> = match format {
            OutputFormat::MarkdownTable => Box::new(MarkdownTableFormatter),
            OutputFormat::MarkdownList => Box::new(MarkdownListFormatter),
            OutputFormat::Json => Box::new(JsonFormatter),
            OutputFormat::Toml => Box::new(TomlFormatter),
            OutputFormat::Yaml => Box::new(YamlFormatter),
        };

        Self { formatter, writer }
    }

    pub fn write(&mut self, deps: &[DependencyInfo]) -> Result<()> {
        let content = self.formatter.format(deps)?;
        self.writer.write_all(content.as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }
}

impl From<(&str, &Source)> for DependencyInfo {
    fn from((name, source): (&str, &Source)) -> Self {
        match source {
            Source::GitHub { owner, repo, stars } => Self {
                name: name.to_string(),
                description: None,
                source_type: "GitHub".to_string(),
                source_url: Some(format!("https://github.com/{}/{}", owner, repo)),
                stats: DependencyStats {
                    stars: *stars,
                    downloads: None,
                },
                failed: false,
                error_message: None,
            },
            Source::CratesIo { downloads, .. } => Self {
                name: name.to_string(),
                description: None,
                source_type: "crates.io".to_string(),
                source_url: Some(format!("https://crates.io/crates/{}", name)),
                stats: DependencyStats {
                    stars: None,
                    downloads: *downloads,
                },
                failed: false,
                error_message: None,
            },
            Source::Link { url } => Self {
                name: name.to_string(),
                description: None,
                source_type: "Source".to_string(),
                source_url: Some(url.clone()),
                stats: DependencyStats {
                    stars: None,
                    downloads: None,
                },
                failed: false,
                error_message: None,
            },
            Source::Other { description } => Self {
                name: name.to_string(),
                description: Some(description.clone()),
                source_type: description.clone(),
                source_url: None,
                stats: DependencyStats {
                    stars: None,
                    downloads: None,
                },
                failed: false,
                error_message: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    use super::*;

    #[test]
    fn test_markdown_table_formatter() {
        let deps = vec![DependencyInfo {
            name: "serde".to_string(),
            description: Some(
                "A data interchange format with a strong focus on simplicity and usability."
                    .to_string(),
            ),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
        }];

        let formatter = MarkdownTableFormatter;
        let result = formatter.format(&deps).unwrap();
        println!("{}", result);
        assert!(result.contains("| serde |"));
        assert!(result.contains(" 🌟 1000"));
    }

    #[test]
    fn test_write_to_memory() {
        let deps = vec![DependencyInfo {
            name: "serde".to_string(),
            description: Some(
                "A data interchange format with a strong focus on simplicity and usability."
                    .to_string(),
            ),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
        }];

        let mut buffer = Vec::new();
        let mut manager = OutputManager::new(OutputFormat::MarkdownTable, &mut buffer);
        manager.write(&deps).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("| serde |"));
        assert!(output.contains("🌟 1000"));
    }

    #[test]
    fn test_write_to_file() -> Result<()> {
        let deps = vec![DependencyInfo {
            name: "serde".to_string(),
            description: Some(
                "A data interchange format with a strong focus on simplicity and usability."
                    .to_string(),
            ),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
        }];

        let temp_dir = assert_fs::TempDir::new()?;
        let file_path = temp_dir.path().join("test-output.md");
        let file = std::fs::File::create(&file_path)?;

        let mut manager = OutputManager::new(OutputFormat::MarkdownTable, file);
        manager.write(&deps)?;

        let content = std::fs::read_to_string(&file_path)?;
        assert!(content.contains("| serde |"));
        assert!(content.contains("🌟 1000"));

        Ok(())
    }

    #[test]
    fn test_write_to_output_writer_stdout() -> Result<()> {
        let deps = vec![DependencyInfo {
            name: "serde".to_string(),
            description: Some(
                "A data interchange format with a strong focus on simplicity and usability."
                    .to_string(),
            ),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
        }];

        let config = Config::default();

        let mut output = config.get_output_writer()?;
        let mut manager = OutputManager::new(OutputFormat::MarkdownTable, &mut output);
        manager.write(&deps)?;

        Ok(())
    }

    #[test]
    fn test_write_to_output_writer_stdout_with_failed_dependency() -> Result<()> {
        let deps = vec![DependencyInfo {
            name: "serde".to_string(),
            description: Some(
                "A data interchange format with a strong focus on simplicity and usability."
                    .to_string(),
            ),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: true,
            error_message: Some("Failed to fetch repository info".to_string()),
        }];

        let config = Config::default();

        let mut output = config.get_output_writer()?;
        let mut manager = OutputManager::new(OutputFormat::MarkdownTable, &mut output);
        manager.write(&deps)?;

        Ok(())
    }

    #[test]
    fn test_write_to_output_writer_file() -> Result<()> {
        let deps = vec![DependencyInfo {
            name: "serde".to_string(),
            description: Some(
                "A data interchange format with a strong focus on simplicity and usability."
                    .to_string(),
            ),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
        }];

        let temp_dir = assert_fs::TempDir::new()?;
        let file_path = temp_dir.path().join("test-output.md");

        let mut config = Config::default();
        config.output = Some(file_path.clone());

        let mut output = config.get_output_writer()?;
        let mut manager = OutputManager::new(OutputFormat::MarkdownTable, &mut output);
        manager.write(&deps)?;

        let content = std::fs::read_to_string(&file_path)?;
        assert!(content.contains("| serde |"));
        assert!(content.contains("🌟 1000"));

        Ok(())
    }
}
