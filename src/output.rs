use anyhow::Result;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::{io::Write, str::FromStr};
use tracing::instrument;

use crate::{errors::AppError, sources::Source};

/// 定义输出格式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    MarkdownTable,
    MarkdownList,
    Csv,
    Json,
    // Toml,
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
            // "toml" => Self::Toml,
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
        let s = s.trim().to_lowercase();

        if s.is_empty() {
            return Err(AppError::InvalidDependencyKind(s.to_string()).into());
        }

        if s == t!("output.normal").to_lowercase() {
            Ok(Self::Normal)
        } else if s == t!("output.development").to_lowercase() {
            Ok(Self::Development)
        } else if s == t!("output.build").to_lowercase() {
            Ok(Self::Build)
        } else if s == t!("output.unknown").to_lowercase() {
            Ok(Self::Unknown)
        } else {
            Err(AppError::InvalidDependencyKind(format!(
                "{}",
                t!("output.invalid_dependency_kind", kind = s.to_string())
            ))
            .into())
        }
    }
}

impl std::fmt::Display for DependencyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DependencyKind::Normal => t!("output.normal"),
            DependencyKind::Development => t!("output.development"),
            DependencyKind::Build => t!("output.build"),
            DependencyKind::Unknown => t!("output.unknown"),
        };

        write!(f, "{}", s)
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

impl DependencyKind {
    pub fn to_md_table_header(&self) -> impl AsRef<str> {
        match self {
            DependencyKind::Normal => format!("| 🔍 | {} | | | | |", t!("output.normal")),
            DependencyKind::Development => format!("| 🔧 | {} | | | | |", t!("output.development")),
            DependencyKind::Build => format!("| 🔨 | {} | | | | |", t!("output.build")),
            DependencyKind::Unknown => format!("| ❓ | {} | | | | |", t!("output.unknown")),
        }
    }

    pub fn to_md_list_header(&self) -> impl AsRef<str> {
        let s = match self {
            DependencyKind::Normal => t!("output.normal"),
            DependencyKind::Development => t!("output.development"),
            DependencyKind::Build => t!("output.build"),
            DependencyKind::Unknown => t!("output.unknown"),
        };

        format!("## {}", s)
    }

    #[instrument]
    pub fn try_from_table_header(s: &str) -> Result<Self> {
        let s = s.trim();
        let columns: Vec<&str> = s.split("|").collect();
        if columns.len() < 2 {
            return Err(AppError::InvalidTableHeader(s.to_string()).into());
        }

        let kind = columns[1].trim();
        Ok(Self::from_str(kind)?)
    }

    #[instrument]
    pub fn try_from_list_header(s: &str) -> Result<Self> {
        let s = s.trim_start_matches("## ");
        let s = s.trim_end_matches("\n");
        Ok(Self::from_str(s)?)
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
        let columns: Vec<&str> = line.split(",").map(|s| s.trim()).collect();

        if columns.len() != header_num {
            return Err(AppError::InvalidCsvContent(line.to_string()).into());
        }

        let name = columns[0].to_string();
        let description = if let Some(description) = Self::option_from_str::<String>(columns[1])? {
            Some(description.replace(";", ","))
        } else {
            None
        };
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

    pub fn try_from_md_table_line(line: &str, dependency_kind: &DependencyKind) -> Result<Self> {
        let columns: Vec<&str> = line
            .trim_matches(['|', ' ', '\n'])
            .split("|")
            .map(|s| s.trim())
            .collect();

        if columns.len() != MarkdownTableFormatter::get_column_num() {
            return Err(AppError::InvalidTableLine(line.to_string()).into());
        }

        let name = columns[0].to_string();
        let description = DependencyInfo::option_from_str(columns[1])?;
        let dependency_kind = dependency_kind.clone();
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

    pub fn try_from_md_list_line(line: &str, dependency_kind: &DependencyKind) -> Result<Self> {
        // ## Development
        // - serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅
        // the output code is like this:
        // output.push_str(&format!(
        //     "- {} : {} - {} {} ({}) {}\n",
        //     dep.name, description, crates_link, source_link, stats, status
        // ));

        // we need to parse the line to get the name, description, crates_link, source_link, stats, status
        let parts: Vec<&str> = line.split(" - ").collect();

        if parts.len() != 2 {
            return Err(AppError::InvalidListLine(line.to_string()).into());
        }

        let parts0: Vec<&str> = parts[0].split(" : ").collect();

        if parts0.len() != 2 {
            return Err(AppError::InvalidListLine(line.to_string()).into());
        }

        let name = parts0[0].trim_start_matches('-').trim().to_string();
        let description = DependencyInfo::option_from_str(parts0[1])?;
        let dependency_kind = dependency_kind.clone();

        let parts1: Vec<&str> = parts[1].split(")").collect();

        if parts1.len() != 4 {
            return Err(AppError::InvalidListLine(line.to_string()).into());
        }

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

impl dyn Formatter {
    pub fn new(format: OutputFormat) -> Result<Box<dyn Formatter>> {
        Ok(match format {
            OutputFormat::MarkdownTable => Box::new(MarkdownTableFormatter),
            OutputFormat::MarkdownList => Box::new(MarkdownListFormatter),
            OutputFormat::Csv => Box::new(CsvFormatter),
            OutputFormat::Json => Box::new(JsonFormatter),
            OutputFormat::Yaml => Box::new(YamlFormatter),
        })
    }
}

/// Markdown 表格格式化器
pub struct MarkdownTableFormatter;

impl MarkdownTableFormatter {
    fn get_header() -> impl AsRef<str> {
        format!(
            "| {} | {} | {} | {} | {} | {} |",
            t!("output.name"),
            t!("output.description"),
            t!("output.crates_link"),
            t!("output.source_link"),
            t!("output.stats"),
            t!("output.status")
        )
    }

    fn get_column_num() -> usize {
        MarkdownTableFormatter::get_header()
            .as_ref()
            .split('|')
            .count()
            - 2
    }

    fn get_separator() -> impl AsRef<str> {
        let column_num = MarkdownTableFormatter::get_column_num();
        format!("|{}", "---|".repeat(column_num))
    }

    /// 从文本内容中提取第一个合法的 Markdown 表格
    ///
    /// # 参数
    /// * `content` - 要搜索的文本内容
    ///
    /// # 返回值
    /// * `Option<&str>` - 找到的第一个合法 Markdown 表格，如果没有找到则返回 None
    fn get_first_md_table(content: &str) -> Option<&str> {
        // 按行分割文本
        let lines: Vec<&str> = content.lines().collect();

        // 至少需要两行才能形成一个表格（表头和分隔符）
        if lines.len() < 2 {
            return None;
        }

        for i in 0..lines.len() - 1 {
            // 检查当前行是否可能是表头
            let header_line = lines[i].trim();
            if !header_line.starts_with('|') && !header_line.contains('|') {
                continue;
            }

            // 检查下一行是否是有效的分隔符行
            let separator_line = lines[i + 1].trim();
            if !MarkdownTableFormatter::is_valid_separator(separator_line) {
                continue;
            }

            // 计算列数
            let header_columns = MarkdownTableFormatter::count_columns(header_line);
            let separator_columns = MarkdownTableFormatter::count_columns(separator_line);

            // 检查表头和分隔符的列数是否匹配
            if header_columns != separator_columns {
                continue;
            }

            // 找到表格的结束位置
            let mut end_idx = i + 2;
            while end_idx < lines.len() {
                let row = lines[end_idx].trim();
                // 如果行为空或不包含'|'，则表格结束
                if row.is_empty() || (!row.starts_with('|') && !row.contains('|')) {
                    break;
                }

                // 检查数据行的列数是否与表头一致
                if MarkdownTableFormatter::count_columns(row) != header_columns {
                    break;
                }

                end_idx += 1;
            }

            // 如果只有表头和分隔符，也是有效的表格
            if end_idx >= i + 2 {
                // 计算表格在原始文本中的起始和结束位置
                let start_pos = content.find(lines[i]).unwrap();
                let end_line_start = content.find(lines[end_idx - 1]).unwrap();
                let end_pos = end_line_start + lines[end_idx - 1].len();

                return Some(&content[start_pos..end_pos]);
            }
        }

        None
    }

    /// 检查一行是否是有效的 Markdown 表格分隔符
    fn is_valid_separator(line: &str) -> bool {
        if !line.contains('|') {
            return false;
        }

        // 分割分隔符行
        let cells = MarkdownTableFormatter::split_table_row(line);

        // 检查每个分隔符单元格是否有效
        for cell in cells {
            let trimmed = cell.trim();
            if trimmed.is_empty() {
                continue;
            }

            // 分隔符必须只包含 '-', ':', 和空格
            if !trimmed.chars().all(|c| c == '-' || c == ':' || c == ' ') {
                return false;
            }

            // 分隔符必须至少包含一个 '-'
            if !trimmed.contains('-') {
                return false;
            }
        }

        true
    }

    /// 计算表格行中的列数
    fn count_columns(line: &str) -> usize {
        MarkdownTableFormatter::split_table_row(line).len()
    }

    /// 分割表格行为单元格
    fn split_table_row(line: &str) -> Vec<&str> {
        let line = line.trim();
        let mut cells = Vec::new();

        // 处理以'|'开头和结尾的情况
        let processed_line = if line.starts_with('|') {
            if line.ends_with('|') {
                &line[1..line.len() - 1]
            } else {
                &line[1..]
            }
        } else if line.ends_with('|') {
            &line[..line.len() - 1]
        } else {
            line
        };

        // 分割单元格
        for cell in processed_line.split('|') {
            cells.push(cell);
        }

        cells
    }
}

impl Formatter for MarkdownTableFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let mut output = String::new();

        // 表头
        output.push_str(&format!(
            "\n{}\n",
            MarkdownTableFormatter::get_header().as_ref()
        ));
        output.push_str(&format!(
            "{}\n",
            MarkdownTableFormatter::get_separator().as_ref()
        ));

        let dep_kind_order = vec![
            DependencyKind::Normal,
            DependencyKind::Development,
            DependencyKind::Build,
            DependencyKind::Unknown,
        ];

        for kind in dep_kind_order {
            let mut show_header = true;
            let deps = take_sort_dependencies(deps, &kind);

            let header = kind.to_md_table_header();

            for dep in deps {
                if show_header {
                    output.push_str(&format!("{}\n", header.as_ref()));
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

    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        // 1. find the markdown table header and separator
        // 2. find the DependencyKind row and store it into a variable to pass to the next step
        // 3. parse the markdown table row into DependencyInfo struct
        let first_md_table = MarkdownTableFormatter::get_first_md_table(content);
        if first_md_table.is_none() {
            return Ok(vec![]);
        }

        let md_table = first_md_table.unwrap();

        let mut deps = vec![];
        let mut dependency_kind = DependencyKind::Unknown;
        // skip the first two lines (header and separator)
        for line in md_table.lines().skip(2) {
            let line = line.trim();
            match line {
                line if line.contains(DependencyKind::Normal.to_md_table_header().as_ref()) => {
                    dependency_kind = DependencyKind::Normal;
                    continue;
                }
                line if line
                    .contains(DependencyKind::Development.to_md_table_header().as_ref()) =>
                {
                    dependency_kind = DependencyKind::Development;
                    continue;
                }
                line if line.contains(DependencyKind::Build.to_md_table_header().as_ref()) => {
                    dependency_kind = DependencyKind::Build;
                    continue;
                }
                line if line.contains(DependencyKind::Unknown.to_md_table_header().as_ref()) => {
                    dependency_kind = DependencyKind::Unknown;
                    continue;
                }
                _ => {}
            };

            let dep = DependencyInfo::try_from_md_table_line(line, &dependency_kind)?;
            deps.push(dep);
        }

        Ok(deps)
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

impl MarkdownListFormatter {
    fn get_header() -> impl AsRef<str> {
        format!("# {}", t!("output.dependencies"))
    }

    fn get_first_md_list(content: &str) -> Option<&str> {
        // 使用更精确的正则表达式匹配 Markdown 标题
        let regex = regex::Regex::new(r"(?m)^(#|##) .+$").ok()?;

        // 找到所有标题
        let headers: Vec<_> = regex.find_iter(content).collect();

        if headers.len() < 2 {
            tracing::warn!(
                "{}",
                t!("output.invalid_list_header_num", num = headers.len())
            );
            return None;
        }

        // 找到第一个主标题 (# 开头)
        let start_header_idx = headers
            .iter()
            .position(|m| content[m.start()..].starts_with("# "))?;
        let start_header = headers[start_header_idx];
        let start_pos = start_header.start();

        // 找到列表的结束位置
        // 可能是下一个同级标题或文档结束
        let end_pos = headers
            .iter()
            .skip(start_header_idx + 1)
            .find(|m| content[m.start()..].starts_with("# "))
            .map(|m| m.start())
            .unwrap_or_else(|| {
                // 如果没有下一个主标题，查找可能的其他结束标记
                // 例如 "### " 开头的标题或文档结束
                content[start_pos..]
                    .find("\n### ")
                    .map(|pos| start_pos + pos)
                    .unwrap_or(content.len())
            });

        // 提取列表内容
        let list_content = &content[start_pos..end_pos];

        // 验证提取的内容是否包含有效的列表项
        let lines_after_header = list_content
            .lines()
            .skip(1) // 跳过标题行
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();

        // 检查是否至少有一个子标题和一个列表项
        if lines_after_header
            .iter()
            .any(|line| line.starts_with("## "))
            && lines_after_header.iter().any(|line| {
                DependencyInfo::try_from_md_list_line(line, &DependencyKind::Unknown).is_ok()
            })
        {
            Some(list_content)
        } else {
            tracing::warn!("{}", t!("output.no_valid_list_items_found"));
            None
        }
    }
}

impl Formatter for MarkdownListFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let mut output = String::new();
        output.push_str(&format!(
            "\n{}\n",
            MarkdownListFormatter::get_header().as_ref()
        ));

        let dep_kind_order = vec![
            DependencyKind::Normal,
            DependencyKind::Development,
            DependencyKind::Build,
            DependencyKind::Unknown,
        ];

        for kind in dep_kind_order {
            let mut show_header = true;
            let deps = take_sort_dependencies(deps, &kind);

            let header = kind.to_md_list_header();

            for dep in deps {
                if show_header {
                    output.push_str(&format!("\n{}\n", header.as_ref()));
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

    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        // 1. find the markdown list header and separator
        // 2. find the DependencyKind row and store it into a variable to pass to the next step
        // 3. parse the markdown table row into DependencyInfo struct
        let first_md_list = MarkdownListFormatter::get_first_md_list(content);
        // dbg!(&first_md_list);
        if first_md_list.is_none() {
            return Ok(vec![]);
        }

        let md_list = first_md_list.unwrap();

        let mut deps = vec![];
        let mut dependency_kind = DependencyKind::Unknown;
        // skip the first two lines (header and separator)
        for line in md_list.lines() {
            let line = line.trim();
            match line {
                line if line.contains(DependencyKind::Normal.to_md_list_header().as_ref()) => {
                    dependency_kind = DependencyKind::Normal;
                    continue;
                }
                line if line.contains(DependencyKind::Development.to_md_list_header().as_ref()) => {
                    dependency_kind = DependencyKind::Development;
                    continue;
                }
                line if line.contains(DependencyKind::Build.to_md_list_header().as_ref()) => {
                    dependency_kind = DependencyKind::Build;
                    continue;
                }
                line if line.contains(DependencyKind::Unknown.to_md_list_header().as_ref()) => {
                    dependency_kind = DependencyKind::Unknown;
                    continue;
                }
                _ => {}
            };

            let dep = DependencyInfo::try_from_md_list_line(line, &dependency_kind);
            if let Ok(dep) = dep {
                deps.push(dep);
            } else {
                tracing::warn!("{}", t!("output.failed_to_parse_list_line", line = line));
                continue;
            }
        }

        Ok(deps)
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
// pub struct TomlFormatter;

// impl Formatter for TomlFormatter {
//     fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
//         Ok(toml::to_string_pretty(deps)?)
//     }

//     fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
//         Ok(toml::from_str(content)?)
//     }
// }

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
    fn get_header() -> impl AsRef<str> {
        t!("output.csv_header").replace("，", ",")
    }

    fn column_num() -> usize {
        CsvFormatter::get_header().as_ref().split(",").count()
    }
}

impl Formatter for CsvFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let header = CsvFormatter::get_header();
        let mut output = String::new();
        output.push_str(&format!("\n{}\n", header.as_ref()));

        for dep in deps {
            let (name, description, crates_link, source_link, stats, status) = dep.to_strings();
            let description = description.replace(",", ";");
            let dependency_kind = dep.dependency_kind.to_string();

            output.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                name, description, dependency_kind, crates_link, source_link, stats, status,
            ));
        }

        Ok(output)
    }

    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        let mut lines = content.lines().filter(|line| !line.trim().is_empty()); // skip empty lines

        let header = lines.next();

        if header.is_none() {
            return Err(AppError::InvalidCsvContent(content.to_string()).into());
        }

        let columns = header.unwrap().split(",").collect::<Vec<_>>();
        let column_num = columns.len();

        if column_num != CsvFormatter::column_num() {
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
            // OutputFormat::Toml => Box::new(TomlFormatter),
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
        assert!(output.contains(&format!("{}", t!("output.unknown"))));
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
        assert!(content.contains(&format!("{}", t!("output.development"))));

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
        assert!(content.contains(&format!("{}", t!("output.normal"))));

        Ok(())
    }

    #[test]
    fn test_md_table_func() -> Result<()> {
        let header = MarkdownTableFormatter::get_header();
        let column_num = MarkdownTableFormatter::get_column_num();
        let separator = MarkdownTableFormatter::get_separator();
        dbg!(header.as_ref(), column_num, separator.as_ref());
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
    #[ignore = "skip csv test on test, only run on demand (manual test or cargo test -- --skip _zh --include-ignored)"]
    fn test_try_from_csv_line_en() -> Result<()> {
        rust_i18n::set_locale("en");
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
    #[ignore = "skip csv test on test, only run on demand (manual test or cargo test -- --skip _en --include-ignored)"]
    fn test_try_from_csv_line_zh() -> Result<()> {
        rust_i18n::set_locale("zh");
        const LINE: &str = "serde,serde 是一个强大的数据序列化框架;用于 Rust，普通，[crates.io](https://crates.io/crates/serde),[GitHub](https://github.com/serde-rs/serde),🌟 1000,✅,";
        let line = LINE.replace("，", ",");

        let header_num = line.split(",").count();

        let dep = DependencyInfo::try_from_csv_line(&line, header_num)?;
        assert_eq!(dep.name, "serde");
        assert_eq!(
            dep.description,
            Some(
                "serde 是一个强大的数据序列化框架，用于 Rust"
                    .replace("，", ",")
                    .to_string()
            )
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

        let dep = DependencyInfo::try_from_md_list_line(LINE, &DependencyKind::Normal)?;
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

        let test_line = "- anyhow : Flexible concrete Error type built on std::error::Error - [anyhow](https://crates.io/crates/anyhow) [GitHub](https://github.com/dtolnay/anyhow) (❓) ✅";
        let dep = DependencyInfo::try_from_md_list_line(test_line, &DependencyKind::Development)?;
        assert_eq!(dep.name, "anyhow");
        assert_eq!(
            dep.description,
            Some("Flexible concrete Error type built on std::error::Error".to_string())
        );
        assert_eq!(dep.dependency_kind, DependencyKind::Development);
        assert_eq!(
            dep.crate_url,
            Some("https://crates.io/crates/anyhow".to_string())
        );
        assert_eq!(dep.source_type, "GitHub");
        assert_eq!(
            dep.source_url,
            Some("https://github.com/dtolnay/anyhow".to_string())
        );
        assert_eq!(dep.stats.stars, None);
        assert_eq!(dep.stats.downloads, None);
        assert!(!dep.failed);
        Ok(())
    }

    #[test]
    fn test_try_from_md_table_line() -> Result<()> {
        const LINE: &str = "| anyhow | Flexible concrete Error type built on std::error::Error | [anyhow](https://crates.io/crates/anyhow) | [GitHub](https://github.com/dtolnay/anyhow) | ❓ | ✅ |";

        let dep = DependencyInfo::try_from_md_table_line(LINE, &DependencyKind::Normal)?;
        assert_eq!(dep.name, "anyhow");
        assert_eq!(
            dep.description,
            Some("Flexible concrete Error type built on std::error::Error".to_string())
        );
        assert_eq!(dep.dependency_kind, DependencyKind::Normal);
        assert_eq!(
            dep.crate_url,
            Some("https://crates.io/crates/anyhow".to_string())
        );
        assert_eq!(dep.source_type, "GitHub");
        assert_eq!(
            dep.source_url,
            Some("https://github.com/dtolnay/anyhow".to_string())
        );
        assert_eq!(dep.stats.stars, None);
        assert_eq!(dep.stats.downloads, None);
        assert!(!dep.failed);
        assert_eq!(dep.error_message, None);

        Ok(())
    }

    const TEST_MD_TABLE: &str = r"hfshdfhsfjsdjfgg
    | 名称 | 描述 |  |
    |:---:|:---:|:---:|
    | hello | world | NIhao |
    | 名称 | 描述 | No 1 end|
    |:---:|:---:|| 名称 | 描述 | | |
    |:---:|:---:|
    | 名称 | 描述 |  | 第二个 | 
    |:---:|:---:|:---:|:---:|
    | hello | world | NIhao | :---:|
    |:---:|:---:|| 名称 |
    |:---:|:---:|
    | hello | world |
    | 名称 | 描述 | | |
    |:---:|:---:|
    ";
    #[test]
    fn test_find_md_start_end_pos() -> Result<()> {
        let md_table = MarkdownTableFormatter::get_first_md_table(TEST_MD_TABLE)
            .ok_or_else(|| anyhow::anyhow!("no table found"))?;
        dbg!(md_table);
        Ok(())
    }

    const TEST_MD_LIST: &str = "# 依赖项
## Normal
## Development
- 1serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅
### 开发依赖
- 2serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅
- 3serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅
- 4serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅
## Build
- 5serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅
- 6serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅
- 7serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅
- 8serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅
## Unknown
- 9serde : serde is a powerful data serialization framework for Rust - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 1000 📦 100) ✅ 
### 未知依赖";

    #[test]
    fn test_find_md_list() -> Result<()> {
        let md_list_content = MarkdownListFormatter::get_first_md_list(TEST_MD_LIST);
        assert!(md_list_content.is_some());
        dbg!(md_list_content.unwrap());
        Ok(())
    }

    #[test]
    #[ignore = "skip md table test on test, only run on demand (manual test or cargo test -- --skip _zh --include-ignored)"]
    fn test_parse_md_table_en() -> Result<()> {
        rust_i18n::set_locale("en");
        let content = std::fs::read_to_string("./assets/output/THANKU_table_en.md")?;
        let deps = MarkdownTableFormatter.parse(&content)?;
        let mut output = Vec::new(); // Change String to Vec<u8>
        let mut manager = OutputManager::new(OutputFormat::MarkdownTable, &mut output); // Pass &mut output
        manager.write(&deps)?;
        let output_str = String::from_utf8(output).unwrap();
        // std::fs::write("./assets/output/THANKU_table_parsed.csv", &output_str)?;
        assert_eq!(output_str, content);
        Ok(())
    }

    #[test]
    #[ignore = "skip md list test on auto-test, only run on demand (manual test or cargo test -- --skip _zh --include-ignored)"]
    fn test_parse_md_list_en() -> Result<()> {
        rust_i18n::set_locale("en");
        let content = std::fs::read_to_string("./assets/output/THANKU_list_en.md")?;
        let deps = MarkdownListFormatter.parse(&content)?;
        let mut output = Vec::new(); // Change String to Vec<u8>
        let mut manager = OutputManager::new(OutputFormat::MarkdownList, &mut output); // Pass &mut output
        manager.write(&deps)?;
        let output_str = String::from_utf8(output).unwrap();
        // std::fs::write("./assets/output/THANKU_list_parsed.md", &output_str)?;
        let output_str = format!("{}\n## Unknown\n\n### Failed Test", output_str);
        assert_eq!(output_str, content);
        Ok(())
    }

    #[test]
    #[ignore = "skip csv test on test, only run on demand (manual test or cargo test -- --skip _zh --include-ignored)"]
    fn test_parse_csv_en() -> Result<()> {
        rust_i18n::set_locale("en");
        let content = std::fs::read_to_string("./assets/output/THANKU_en.csv")?;
        let deps = CsvFormatter.parse(&content)?;
        let mut output = Vec::new(); // Change String to Vec<u8>
        let mut manager = OutputManager::new(OutputFormat::Csv, &mut output); // Pass &mut output
        manager.write(&deps)?;
        let output_str = String::from_utf8(output).unwrap();
        // std::fs::write("./assets/output/THANKU_table_parsed2.csv", &output_str)?;
        assert_eq!(output_str, content);
        Ok(())
    }

    #[test]
    #[ignore = "skip csv test on test, only run on demand (manual test or cargo test -- --skip _en --include-ignored)"]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: Invalid dependency kind: ❌ 无效的依赖类型：normal"
    )]
    fn test_parse_csv_failed_zh() {
        rust_i18n::set_locale("zh");
        let content = std::fs::read_to_string("./assets/output/THANKU_en.csv").unwrap();
        let deps = CsvFormatter.parse(&content).unwrap();
        let mut output = Vec::new(); // Change String to Vec<u8>
        let mut manager = OutputManager::new(OutputFormat::Csv, &mut output); // Pass &mut output
        manager.write(&deps).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        // std::fs::write("./assets/output/THANKU_table_parsed2.csv", &output_str)?;
        assert_eq!(output_str, content);
    }

    #[test]
    #[ignore = "skip json test on test, only run on demand (manual test or cargo test -- --skip _zh --include-ignored)"]
    fn test_parse_json_en() -> Result<()> {
        rust_i18n::set_locale("en");
        let content = std::fs::read_to_string("./assets/output/THANKU_json_en.json")?;
        let deps = JsonFormatter.parse(&content)?;
        let mut output = Vec::new(); // Change String to Vec<u8>
        let mut manager = OutputManager::new(OutputFormat::Json, &mut output); // Pass &mut output
        manager.write(&deps)?;
        let output_str = String::from_utf8(output).unwrap();
        // std::fs::write("./assets/output/THANKU_json_en.json", &output_str)?;
        assert_eq!(output_str, content);
        Ok(())
    }

    #[test]
    #[ignore = "skip yaml test on test, only run on demand (manual test or cargo test -- --skip _zh --include-ignored)"]
    fn test_parse_yaml_en() -> Result<()> {
        rust_i18n::set_locale("en");
        let content = std::fs::read_to_string("./assets/output/THANKU_yaml_en.yaml")?;
        let deps = YamlFormatter.parse(&content)?;
        let mut output = Vec::new(); // Change String to Vec<u8>
        let mut manager = OutputManager::new(OutputFormat::Yaml, &mut output); // Pass &mut output
        manager.write(&deps)?;
        let output_str = String::from_utf8(output).unwrap();
        // std::fs::write("./assets/output/THANKU_yaml_en.yaml", &output_str)?;
        assert_eq!(output_str, content);
        Ok(())
    }
}
