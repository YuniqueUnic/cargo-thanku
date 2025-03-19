use anyhow::Result;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::{io::Write, str::FromStr};

use crate::{errors::AppError, sources::Source};

/// 定义输出格式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    MarkdownTable,
    MarkdownList,
    Csv,
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
        Ok(match s.to_lowercase().as_str() {
            "markdown-table" => Self::MarkdownTable,
            "markdown-list" => Self::MarkdownList,
            "csv" => Self::Csv,
            "json" => Self::Json,
            "toml" => Self::Toml,
            "yaml" => Self::Yaml,
            _ => return Err(AppError::InvalidOutputFormat(s.to_string())),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum DependencyKind {
    #[default]
    Normal,
    Development,
    Build,
    Unknown,
}

impl std::str::FromStr for DependencyKind {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "normal" => Self::Normal,
            "development" => Self::Development,
            "build" => Self::Build,
            _ => return Err(AppError::InvalidDependencyKind(s.to_string())),
        })
    }
}

impl std::fmt::Display for DependencyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<cargo_metadata::DependencyKind> for DependencyKind {
    fn from(kind: cargo_metadata::DependencyKind) -> Self {
        match kind {
            cargo_metadata::DependencyKind::Normal => Self::Normal,
            cargo_metadata::DependencyKind::Development => Self::Development,
            cargo_metadata::DependencyKind::Build => Self::Build,
            _ => Self::Unknown,
        }
    }
}

/// 表示一个依赖项的信息
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DependencyInfo {
    pub name: String,
    pub description: Option<String>,
    pub dependency_kind: DependencyKind,
    pub crate_url: Option<String>,
    pub source_type: String,
    pub source_url: Option<String>,
    pub stats: DependencyStats,
    pub failed: bool,
    pub error_message: Option<String>,
}

#[allow(dead_code)]
impl DependencyInfo {
    pub fn to_strings(&self) -> (String, String, String, String, String, String) {
        let name = self.name.clone();

        let description = match self.description {
            Some(ref description) => description.replace("\n", " "), // 将 description 多行变为一行
            None => "unknown".to_string(),
        };

        let stats = match (self.stats.stars, self.stats.downloads) {
            (Some(stars), _) => format!("🌟 {}", stars),
            (None, Some(downloads)) => format!("📦 {}", downloads),
            _ => "❓".to_string(),
        };

        let status = if self.failed {
            format!("❌ {}", self.error_message.as_deref().unwrap_or("Failed"))
        } else {
            "✅".to_string()
        };

        let crates_link = if let Some(url) = &self.crate_url {
            format!("[{}]({})", self.name, url)
        } else {
            self.name.clone()
        };

        let source_link = if let Some(url) = &self.source_url {
            format!("[{}]({})", self.source_type, url)
        } else {
            self.source_type.clone()
        };

        (name, description, crates_link, source_link, stats, status)
    }

    pub fn try_from_csv_line(line: &str, header_num: usize) -> Result<Self> {
        let columns: Vec<&str> = line.split(",").collect();

        if columns.len() != header_num {
            return Err(AppError::InvalidCsvContent(line.to_string()).into());
        }

        let name = columns[0].to_string();
        let description = Self::option_from_str(columns[1])?;
        let dependency_kind = DependencyKind::from_str(columns[2])?;
        let (_crateio, crate_url) = Self::parse_md_link(columns[3])?;
        let (source_type, source_url) = Self::parse_md_link(columns[4])?;
        let (stars, downloads) = Self::parse_stats(columns[5])?;
        let (failed, error_message) = Self::parse_status(columns[6])?;

        let dep = Self {
            name,
            description,
            dependency_kind,
            crate_url,
            source_type,
            source_url,
            stats: DependencyStats { stars, downloads },
            failed,
            error_message,
        };

        Ok(dep)
    }

    pub fn try_from_md_table_line(line: &str, dependency_kind: DependencyKind) -> Result<Self> {
        let columns: Vec<&str> = line.split("|").collect();

        let name = columns[0].to_string();
        let description = DependencyInfo::option_from_str(columns[1])?;
        let (_, crate_url) = Self::parse_md_link(columns[2])?;
        let (source_type, source_url) = Self::parse_md_link(columns[3])?;
        let (stars, downloads) = Self::parse_stats(columns[4])?;
        let (failed, error_message) = Self::parse_status(columns[5])?;

        let dep = Self {
            name,
            description,
            dependency_kind,
            crate_url,
            source_type,
            source_url,
            stats: DependencyStats { stars, downloads },
            failed,
            error_message,
        };

        Ok(dep)
    }

    pub fn try_from_md_list_line(line: &str, dependency_kind: DependencyKind) -> Result<Self> {
        // ## Development
        // - serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅
        // the output code is like this:
        // output.push_str(&format!(
        //     "- {} : {} - {} {} ({}) {}\n",
        //     dep.name, description, crates_link, source_link, stats, status
        // ));

        // we need to parse the line to get the name, description, crates_link, source_link, stats, status
        let parts: Vec<&str> = line.split(" - ").collect();

        let parts0: Vec<&str> = parts[0].split(":").collect();
        let name = parts0[0].trim_start_matches('-').trim().to_string();
        let description = DependencyInfo::option_from_str(parts0[1])?;

        let parts1: Vec<&str> = parts[1].split(")").collect();
        let (_, crate_url) = Self::parse_md_link(format!("{})", parts1[0]).as_str())?;
        let (source_type, source_url) = Self::parse_md_link(format!("{})", parts1[1]).as_str())?;
        let (stars, downloads) = Self::parse_stats(format!("{})", parts1[2]).as_str())?;
        let (failed, error_message) = Self::parse_status(parts1[3])?;

        let dep = Self {
            name,
            description,
            dependency_kind,
            crate_url,
            source_type,
            source_url,
            stats: DependencyStats { stars, downloads },
            failed,
            error_message,
        };

        Ok(dep)
    }

    const TRIM_PATTERN: [char; 4] = ['[', '(', ' ', ')'];

    fn option_from_str<T: FromStr>(s: &str) -> anyhow::Result<Option<T>>
    where
        <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        let s = s.trim();

        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s.parse()?))
        }
    }

    fn parse_md_link(s: &str) -> Result<(String, Option<String>)> {
        // example: [GitHub](https://github.com/serde-rs/serde)
        // the output code is like this:
        // let source_link = if let Some(url) = &dep.source_url {
        //     format!("[{}]({})", dep.source_type, url)
        // } else {
        //     dep.source_type.clone()
        // };

        // we need to parse the string to get the source_type and source_url
        // the source_type is the text between the first pair of square brackets
        // the source_url is the text between the second pair of square brackets
        // if there is no second pair of square brackets, the source_url is None

        let parts: Vec<&str> = s.split("](").collect();
        let source_type = parts[0]
            .trim_start_matches(&Self::TRIM_PATTERN)
            .trim_end_matches(&Self::TRIM_PATTERN);
        let source_url = if parts.len() > 1 {
            Some(
                parts[1]
                    .trim_start_matches(&Self::TRIM_PATTERN)
                    .trim_end_matches(&Self::TRIM_PATTERN)
                    .to_string(),
            )
        } else {
            None
        };

        Ok((source_type.to_string(), source_url))
    }

    fn parse_stats(s: &str) -> Result<(Option<u32>, Option<u32>)> {
        // the output code is like this:
        // let stats = match (dep.stats.stars, dep.stats.downloads) {
        //     (Some(stars), _) => format!("🌟 {}", stars),
        //     (None, Some(downloads)) => format!("📦 {}", downloads),
        //     _ => "❓".to_string(),
        // };

        // we need to parse the string to get the stars and downloads
        let s = s
            .trim_start_matches(&Self::TRIM_PATTERN)
            .trim_end_matches(&Self::TRIM_PATTERN);

        match s {
            s if s.contains("🌟") && s.contains("📦") => {
                let s = s.replace("🌟", "").replace("📦", "|");
                let parts: Vec<&str> = s.split("|").collect();
                if parts.len() != 2 {
                    return Err(AppError::InvalidStats(s.to_string()).into());
                }
                let stars = parts[0]
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| AppError::InvalidStats(s.to_string()))?;
                let downloads = parts[1]
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| AppError::InvalidStats(s.to_string()))?;
                Ok((Some(stars), Some(downloads)))
            }
            s if s.contains("🌟") => {
                let parts: Vec<&str> = s.split("🌟").collect();
                let stars = parts[1]
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| AppError::InvalidStats(s.to_string()))?;
                Ok((Some(stars), None))
            }
            s if s.contains("📦") => {
                let parts: Vec<&str> = s.split("📦").collect();
                let downloads = parts[1]
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| AppError::InvalidStats(s.to_string()))?;
                Ok((None, Some(downloads)))
            }
            _ => Ok((None, None)),
        }
    }

    fn parse_status(s: &str) -> Result<(bool, Option<String>)> {
        // the output code is like this:
        // let status = if dep.failed {
        //     format!("❌ {}", dep.error_message.as_deref().unwrap_or("Failed"))
        // } else {
        //     "✅".to_string()
        // };

        let s = s
            .trim_start_matches(&Self::TRIM_PATTERN)
            .trim_end_matches(&Self::TRIM_PATTERN);

        match s {
            s if s.contains("✅") => Ok((false, None)),
            s if s.contains("❌") => {
                let parts: Vec<&str> = s.split("❌").collect();
                let error_message = parts[1].trim();

                if error_message.is_empty() {
                    return Ok((true, None));
                }

                Ok((true, Some(error_message.to_string())))
            }
            _ => Err(AppError::InvalidStatus(s.to_string()).into()),
        }
    }
}

/// 依赖项的统计信息
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DependencyStats {
    pub stars: Option<u32>,
    pub downloads: Option<u32>,
}

/// 格式化器特征
pub trait Formatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String>;
    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>>;
}

/// Markdown 表格格式化器
pub struct MarkdownTableFormatter;

impl Formatter for MarkdownTableFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let mut output = String::new();

        // 表头
        output.push_str(&format!(
            "\n| {} | {} | {} | {} | {} | {} |\n",
            t!("output.name"),
            t!("output.description"),
            t!("output.crates_link"),
            t!("output.source_link"),
            t!("output.stats"),
            t!("output.status")
        ));
        output.push_str("|------|--------|--------|-------|-------|--------|\n");

        let dep_kind_order = vec![
            DependencyKind::Normal,
            DependencyKind::Development,
            DependencyKind::Build,
            DependencyKind::Unknown,
        ];

        for kind in dep_kind_order {
            let mut show_header = true;
            let deps = take_sort_dependencies(deps, &kind);

            let header = match kind {
                DependencyKind::Normal => "|🔍|Normal| | | | |\n",
                DependencyKind::Development => "|🔧|Development| | | | |\n",
                DependencyKind::Build => "|🔨|Build| | | | |\n",
                DependencyKind::Unknown => "|❓|Unknown| | | | |\n",
            };

            for dep in deps {
                if show_header {
                    output.push_str(header);
                    show_header = false;
                }
                let (name, description, crates_link, source_link, stats, status) = dep.to_strings();

                output.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} |\n",
                    name, description, crates_link, source_link, stats, status
                ));
            }
        }

        Ok(output)
    }

    // TODO: 实现解析
    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        Ok(vec![])
    }
}

fn take_sort_dependencies<'a>(
    deps: &'a [DependencyInfo],
    kind: &DependencyKind,
) -> Vec<&'a DependencyInfo> {
    let mut filter_sorted_deps = deps
        .iter()
        .filter(|dep| dep.dependency_kind == *kind)
        .collect::<Vec<_>>();
    filter_sorted_deps.sort_by(|a, b| a.name.cmp(&b.name));
    filter_sorted_deps
}

/// Markdown 列表格式化器
pub struct MarkdownListFormatter;

impl Formatter for MarkdownListFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let mut output = String::new();
        output.push_str(&format!("# {}\n\n", t!("output.dependencies")));

        let dep_kind_order = vec![
            DependencyKind::Normal,
            DependencyKind::Development,
            DependencyKind::Build,
            DependencyKind::Unknown,
        ];

        for kind in dep_kind_order {
            let mut show_header = true;
            let deps = take_sort_dependencies(deps, &kind);

            let header = match kind {
                DependencyKind::Normal => t!("output.normal"),
                DependencyKind::Development => t!("output.development"),
                DependencyKind::Build => t!("output.build"),
                DependencyKind::Unknown => t!("output.unknown"),
            };

            for dep in deps {
                if show_header {
                    output.push_str(&format!("\n## {}\n", header));
                    show_header = false;
                }
                let (name, description, crates_link, source_link, stats, status) = dep.to_strings();

                output.push_str(&format!(
                    "- {} : {} - {} {} ({}) {}\n",
                    name, description, crates_link, source_link, stats, status
                ));
            }
        }

        Ok(output)
    }

    // TODO: 实现解析
    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        Ok(vec![])
    }
}

/// JSON 格式化器
pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        Ok(serde_json::to_string_pretty(deps)?)
    }

    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        Ok(serde_json::from_str(content)?)
    }
}

/// TOML 格式化器
pub struct TomlFormatter;

impl Formatter for TomlFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        Ok(toml::to_string_pretty(deps)?)
    }

    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        Ok(toml::from_str(content)?)
    }
}

/// YAML 格式化器
pub struct YamlFormatter;

impl Formatter for YamlFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        Ok(serde_yaml::to_string(deps)?)
    }

    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        Ok(serde_yaml::from_str(content)?)
    }
}

/// CSV 格式化器
pub struct CsvFormatter;

impl CsvFormatter {
    fn get_header(&self) -> String {
        t!("output.csv_header").replace("，", ",")
    }

    fn column_num(&self) -> usize {
        self.get_header().split(",").count()
    }
}

impl Formatter for CsvFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let header = self.get_header();
        let mut output = String::new();
        output.push_str(&format!("{}\n", header));

        for dep in deps {
            let (name, description, crates_link, source_link, stats, failed) = dep.to_strings();
            let dependency_kind = dep.dependency_kind.to_string();

            output.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                name, description, dependency_kind, crates_link, source_link, stats, failed,
            ));
        }

        Ok(output)
    }

    // TODO: 实现解析
    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        let mut lines = content.lines();
        let header = lines.next();

        if header.is_none() {
            return Err(AppError::InvalidCsvContent(content.to_string()).into());
        }

        let columns = header.unwrap().split(",").collect::<Vec<_>>();
        let column_num = columns.len();

        if column_num != self.column_num() {
            return Err(AppError::InvalidCsvContent(content.to_string()).into());
        }

        let mut deps = Vec::new();
        for line in lines {
            let dep = DependencyInfo::try_from_csv_line(line, column_num)?;
            deps.push(dep);
        }

        Ok(deps)
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
            OutputFormat::Csv => Box::new(CsvFormatter),
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
                crate_url: Some(format!("https://crates.io/crates/{}", name)),
                source_type: "GitHub".to_string(),
                source_url: Some(format!("https://github.com/{}/{}", owner, repo)),
                stats: DependencyStats {
                    stars: *stars,
                    downloads: None,
                },
                failed: false,
                error_message: None,
                dependency_kind: DependencyKind::Normal,
            },
            Source::CratesIo { downloads, .. } => Self {
                name: name.to_string(),
                description: None,
                crate_url: Some(format!("https://crates.io/crates/{}", name)),
                source_type: "crates.io".to_string(),
                source_url: None,
                stats: DependencyStats {
                    stars: None,
                    downloads: *downloads,
                },
                failed: false,
                error_message: None,
                dependency_kind: DependencyKind::Normal,
            },
            Source::Link { url } => Self {
                name: name.to_string(),
                description: None,
                crate_url: Some(format!("https://crates.io/crates/{}", name)),
                source_type: "Source".to_string(),
                source_url: Some(url.clone()),
                stats: DependencyStats {
                    stars: None,
                    downloads: None,
                },
                failed: false,
                error_message: None,
                dependency_kind: DependencyKind::Normal,
            },
            Source::Other { description } => Self {
                name: name.to_string(),
                description: Some(description.clone()),
                crate_url: Some(format!("https://crates.io/crates/{}", name)),
                source_type: description.clone(),
                source_url: None,
                stats: DependencyStats {
                    stars: None,
                    downloads: None,
                },
                failed: false,
                error_message: None,
                dependency_kind: DependencyKind::Normal,
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
            crate_url: Some("https://crates.io/crates/serde".to_string()),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
            dependency_kind: DependencyKind::Normal,
        }];

        let formatter = MarkdownTableFormatter;
        let result = formatter.format(&deps).unwrap();
        println!("{}", result);
        assert!(result.contains("| [serde](https://crates.io/crates/serde) |"));
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
            crate_url: Some("https://crates.io/crates/serde".to_string()),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
            dependency_kind: DependencyKind::Unknown,
        }];

        let mut buffer = Vec::new();
        let mut manager = OutputManager::new(OutputFormat::MarkdownTable, &mut buffer);
        manager.write(&deps).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("| [serde](https://crates.io/crates/serde) |"));
        assert!(output.contains("🌟 1000"));
        assert!(output.contains("Unknown"));
    }

    #[test]
    fn test_write_to_file() -> Result<()> {
        let deps = vec![DependencyInfo {
            name: "serde".to_string(),
            description: Some(
                "A data interchange format with a strong focus on simplicity and usability."
                    .to_string(),
            ),
            crate_url: Some("https://crates.io/crates/serde".to_string()),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
            dependency_kind: DependencyKind::Development,
        }];

        let temp_dir = assert_fs::TempDir::new()?;
        let file_path = temp_dir.path().join("test-output.md");
        let file = std::fs::File::create(&file_path)?;

        let mut manager = OutputManager::new(OutputFormat::MarkdownTable, file);
        manager.write(&deps)?;

        let content = std::fs::read_to_string(&file_path)?;
        assert!(content.contains("| [serde](https://crates.io/crates/serde) |"));
        assert!(content.contains("🌟 1000"));
        assert!(content.contains("Development"));

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
            crate_url: Some("https://crates.io/crates/serde".to_string()),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
            dependency_kind: DependencyKind::Normal,
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
            crate_url: Some("https://crates.io/crates/serde".to_string()),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: true,
            error_message: Some("Failed to fetch repository info".to_string()),
            dependency_kind: DependencyKind::Normal,
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
            crate_url: Some("https://crates.io/crates/serde".to_string()),
            source_type: "GitHub".to_string(),
            source_url: Some("https://github.com/serde-rs/serde".to_string()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
            dependency_kind: DependencyKind::Normal,
        }];

        let temp_dir = assert_fs::TempDir::new()?;
        let file_path = temp_dir.path().join("test-output.md");

        let mut config = Config::default();
        config.output = Some(file_path.clone());

        let mut output = config.get_output_writer()?;
        let mut manager = OutputManager::new(OutputFormat::MarkdownTable, &mut output);
        manager.write(&deps)?;

        let content = std::fs::read_to_string(&file_path)?;
        assert!(content.contains("| [serde](https://crates.io/crates/serde) |"));
        assert!(content.contains("🌟 1000"));
        assert!(content.contains("Normal"));

        Ok(())
    }

    #[test]
    fn test_parse_source_link() -> Result<()> {
        let (source_type, source_url) =
            DependencyInfo::parse_md_link("[GitHub](https://github.com/serde-rs/serde)")?;
        assert_eq!(source_type, "GitHub");
        assert_eq!(
            source_url,
            Some("https://github.com/serde-rs/serde".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_parse_status() -> Result<()> {
        let (failed, error_message) = DependencyInfo::parse_status("✅")?;
        assert!(!failed);
        assert_eq!(error_message, None);

        let (failed, error_message) =
            DependencyInfo::parse_status("❌ Unknown error: failed to fetch repository info")?;
        assert!(failed);
        assert_eq!(
            error_message,
            Some("Unknown error: failed to fetch repository info".to_string())
        );

        let (failed, error_message) = DependencyInfo::parse_status("❌ ")?;
        assert!(failed);
        assert_eq!(error_message, None);

        let parse_error = DependencyInfo::parse_status("🍃 test content");
        assert!(parse_error.is_err());

        Ok(())
    }

    #[test]
    fn test_parse_stats() -> Result<()> {
        let (stars, downloads) = DependencyInfo::parse_stats("🌟 1000 📦 100")?;
        assert_eq!(stars, Some(1000));
        assert_eq!(downloads, Some(100));

        let (stars, downloads) = DependencyInfo::parse_stats("🌟 1000")?;
        assert_eq!(stars, Some(1000));
        assert_eq!(downloads, None);

        let (stars, downloads) = DependencyInfo::parse_stats("📦 100")?;
        assert_eq!(stars, None);
        assert_eq!(downloads, Some(100));

        let (stars, downloads) = DependencyInfo::parse_stats("🍃 test content")?;
        assert!(stars.is_none());
        assert!(downloads.is_none());

        Ok(())
    }

    #[test]
    fn test_try_from_csv_line() -> Result<()> {
        const LINE: &str = "serde,serde is a powerful data serialization framework for Rust,normal,[crates.io](https://crates.io/crates/serde),[GitHub](https://github.com/serde-rs/serde),🌟 1000,✅,";

        let header_num = LINE.split(",").count();

        let dep = DependencyInfo::try_from_csv_line(LINE, header_num)?;
        assert_eq!(dep.name, "serde");
        assert_eq!(
            dep.description,
            Some("serde is a powerful data serialization framework for Rust".to_string())
        );
        assert_eq!(dep.dependency_kind, DependencyKind::Normal);
        assert_eq!(
            dep.crate_url,
            Some("https://crates.io/crates/serde".to_string())
        );
        assert_eq!(dep.source_type, "GitHub");
        assert_eq!(
            dep.source_url,
            Some("https://github.com/serde-rs/serde".to_string())
        );
        assert_eq!(dep.stats.stars, Some(1000));
        assert_eq!(dep.stats.downloads, None);
        assert!(!dep.failed);
        assert_eq!(dep.error_message, None);

        Ok(())
    }

    #[test]
    fn test_try_from_md_list_line() -> Result<()> {
        const LINE: &str = "- serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅";

        let dep = DependencyInfo::try_from_md_list_line(LINE, DependencyKind::Normal)?;
        assert_eq!(dep.name, "serde");
        assert_eq!(
            dep.description,
            Some("serde is a powerful data serialization framework for Rust".to_string())
        );
        assert_eq!(dep.dependency_kind, DependencyKind::Normal);
        assert_eq!(
            dep.crate_url,
            Some("https://crates.io/crates/serde".to_string())
        );
        assert_eq!(dep.source_type, "GitHub");
        assert_eq!(
            dep.source_url,
            Some("https://github.com/serde-rs/serde".to_string())
        );
        assert_eq!(dep.stats.stars, Some(1000));
        assert_eq!(dep.stats.downloads, Some(100));
        assert!(!dep.failed);
        assert_eq!(dep.error_message, None);

        Ok(())
    }
}
