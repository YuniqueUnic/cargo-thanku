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
#[derive(Debug, Serialize)]
pub struct DependencyInfo {
    pub name: String,
    pub source_type: String,
    pub source_url: Option<String>,
    pub stats: DependencyStats,
}

/// 依赖项的统计信息
#[derive(Debug, Serialize)]
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
            "| {} | {} | {} |\n",
            t!("output.name"),
            t!("output.source"),
            t!("output.stats")
        ));
        output.push_str("|------|--------|-------|\n");

        // 内容
        for dep in deps {
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

            output.push_str(&format!("| {} | {} | {} |\n", dep.name, source, stats));
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
            let stats = match (dep.stats.stars, dep.stats.downloads) {
                (Some(stars), _) => format!("🌟 {}", stars),
                (None, Some(downloads)) => format!("📦 {}", downloads),
                _ => "❓".to_string(),
            };

            if let Some(url) = &dep.source_url {
                output.push_str(&format!("- [{}]({}) ({})\n", dep.name, url, stats));
            } else {
                output.push_str(&format!("- {} ({})\n", dep.name, stats));
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

/// 输出器特征
pub trait Writer {
    fn write(&mut self, content: &str) -> Result<()>;
}

/// 标准输出写入器
pub struct StdoutWriter;

impl Writer for StdoutWriter {
    fn write(&mut self, content: &str) -> Result<()> {
        print!("{}", content);
        io::stdout().flush()?;
        Ok(())
    }
}

/// 文件写入器
pub struct FileWriter {
    path: std::path::PathBuf,
}

impl FileWriter {
    pub fn new<P: Into<std::path::PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }
}

impl Writer for FileWriter {
    fn write(&mut self, content: &str) -> Result<()> {
        std::fs::write(&self.path, content)?;
        Ok(())
    }
}

/// 输出管理器
pub struct OutputManager {
    formatter: Box<dyn Formatter>,
    writer: Box<dyn Writer>,
}

impl OutputManager {
    pub fn new(format: OutputFormat, writer: Box<dyn Writer>) -> Self {
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
        self.writer.write(&content)
    }
}

impl From<(&str, &Source)> for DependencyInfo {
    fn from((name, source): (&str, &Source)) -> Self {
        match source {
            Source::GitHub { owner, repo, stars } => Self {
                name: name.to_string(),
                source_type: "GitHub".to_string(),
                source_url: Some(format!("https://github.com/{}/{}", owner, repo)),
                stats: DependencyStats {
                    stars: *stars,
                    downloads: None,
                },
            },
            Source::CratesIo { downloads, .. } => Self {
                name: name.to_string(),
                source_type: "crates.io".to_string(),
                source_url: Some(format!("https://crates.io/crates/{}", name)),
                stats: DependencyStats {
                    stars: None,
                    downloads: *downloads,
                },
            },
            Source::Link { url } => Self {
                name: name.to_string(),
                source_type: "Source".to_string(),
                source_url: Some(url.clone()),
                stats: DependencyStats {
                    stars: None,
                    downloads: None,
                },
            },
            Source::Other { description } => Self {
                name: name.to_string(),
                source_type: description.clone(),
                source_url: None,
                stats: DependencyStats {
                    stars: None,
                    downloads: None,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::Source;

    #[test]
    fn test_markdown_table_formatter() {
        let deps = vec![DependencyInfo {
            name: "serde".to_string(),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
        }];

        let formatter = MarkdownTableFormatter;
        let result = formatter.format(&deps).unwrap();
        assert!(result.contains("| serde |"));
        assert!(result.contains("�� 1000"));
    }
}
