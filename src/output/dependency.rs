use std::str::FromStr;

use anyhow::Result;
use rust_i18n::t;
use serde::{Deserialize, Serialize};

use crate::{
    errors::AppError,
    sources::{CratesioClient, Source},
};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum DependencyKind {
    #[default]
    Normal,
    Development,
    Build,
    Unknown,
}

impl DependencyKind {
    pub(crate) fn ordered() -> [DependencyKind; 4] {
        [
            DependencyKind::Normal,
            DependencyKind::Development,
            DependencyKind::Build,
            DependencyKind::Unknown,
        ]
    }

    pub fn to_md_table_header(&self) -> impl AsRef<str> {
        match self {
            DependencyKind::Normal => format!("| 🔍 | {} | | | | |", t!("output.normal")),
            DependencyKind::Development => {
                format!("| 🔧 | {} | | | | |", t!("output.development"))
            }
            DependencyKind::Build => format!("| 🔨 | {} | | | | |", t!("output.build")),
            DependencyKind::Unknown => format!("| ❓ | {} | | | | |", t!("output.unknown")),
        }
    }

    pub fn to_md_list_header(&self) -> impl AsRef<str> {
        let label = match self {
            DependencyKind::Normal => t!("output.normal"),
            DependencyKind::Development => t!("output.development"),
            DependencyKind::Build => t!("output.build"),
            DependencyKind::Unknown => t!("output.unknown"),
        };

        format!("## {}", label)
    }
}

impl FromStr for DependencyKind {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_lowercase();
        if normalized.is_empty() {
            return Err(AppError::InvalidDependencyKind(normalized));
        }

        match normalized.as_str() {
            kind if kind == t!("output.normal").to_lowercase() => Ok(Self::Normal),
            kind if kind == t!("output.development").to_lowercase() => Ok(Self::Development),
            kind if kind == t!("output.build").to_lowercase() => Ok(Self::Build),
            kind if kind == t!("output.unknown").to_lowercase() => Ok(Self::Unknown),
            _ => Err(AppError::InvalidDependencyKind(
                t!("output.invalid_dependency_kind", kind = normalized).to_string(),
            )),
        }
    }
}

impl std::fmt::Display for DependencyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            DependencyKind::Normal => t!("output.normal"),
            DependencyKind::Development => t!("output.development"),
            DependencyKind::Build => t!("output.build"),
            DependencyKind::Unknown => t!("output.unknown"),
        };
        write!(f, "{}", value)
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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DependencyStats {
    pub stars: Option<u32>,
    pub downloads: Option<u32>,
}

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

impl DependencyInfo {
    const TRIM_PATTERN: [char; 4] = ['[', '(', ' ', ')'];
    const MARKDOWN_COLUMNS: usize = 6;

    pub fn failure(
        name: &str,
        dependency_kind: DependencyKind,
        error_message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.to_string(),
            dependency_kind,
            description: None,
            crate_url: Some(CratesioClient::get_crate_url(name)),
            source_type: "Unknown".to_string(),
            source_url: None,
            stats: DependencyStats {
                stars: None,
                downloads: None,
            },
            failed: true,
            error_message: Some(error_message.into()),
        }
    }

    pub fn to_strings(&self) -> (String, String, String, String, String, String) {
        let description = self
            .description
            .as_ref()
            .map(|desc| desc.replace('\n', " "))
            .unwrap_or_else(|| "unknown".to_string());

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

        let crates_link = self
            .crate_url
            .as_ref()
            .map(|url| format!("[{}]({})", self.name, url))
            .unwrap_or_else(|| self.name.clone());

        let source_link = self
            .source_url
            .as_ref()
            .map(|url| format!("[{}]({})", self.source_type, url))
            .unwrap_or_else(|| self.source_type.clone());

        (
            self.name.clone(),
            description,
            crates_link,
            source_link,
            stats,
            status,
        )
    }

    pub fn try_from_csv_line(line: &str, header_num: usize) -> Result<Self> {
        let columns: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if columns.len() != header_num {
            return Err(AppError::InvalidCsvContent(line.to_string()).into());
        }

        let name = columns[0].to_string();
        let description = Self::option_from_str::<String>(columns[1])?.map(|s| s.replace(';', ","));
        let dependency_kind = DependencyKind::from_str(columns[2])?;
        let (_, crate_url) = Self::parse_md_link(columns[3])?;
        let (source_type, source_url) = Self::parse_md_link(columns[4])?;
        let (stars, downloads) = Self::parse_stats(columns[5])?;
        let (failed, error_message) = Self::parse_status(columns[6])?;

        Ok(Self {
            name,
            description,
            dependency_kind,
            crate_url,
            source_type,
            source_url,
            stats: DependencyStats { stars, downloads },
            failed,
            error_message,
        })
    }

    pub fn try_from_md_table_line(line: &str, dependency_kind: &DependencyKind) -> Result<Self> {
        let columns: Vec<&str> = line
            .trim_matches(['|', ' ', '\n'])
            .split('|')
            .map(|s| s.trim())
            .collect();

        if columns.len() != Self::MARKDOWN_COLUMNS {
            return Err(AppError::InvalidTableLine(line.to_string()).into());
        }

        let name = columns[0].to_string();
        let description = Self::option_from_str(columns[1])?;
        let (_, crate_url) = Self::parse_md_link(columns[2])?;
        let (source_type, source_url) = Self::parse_md_link(columns[3])?;
        let (stars, downloads) = Self::parse_stats(columns[4])?;
        let (failed, error_message) = Self::parse_status(columns[5])?;

        Ok(Self {
            name,
            description,
            dependency_kind: dependency_kind.clone(),
            crate_url,
            source_type,
            source_url,
            stats: DependencyStats { stars, downloads },
            failed,
            error_message,
        })
    }

    pub fn try_from_md_list_line(line: &str, dependency_kind: &DependencyKind) -> Result<Self> {
        let parts: Vec<&str> = line.split(" - ").collect();
        if parts.len() != 2 {
            return Err(AppError::InvalidListLine(line.to_string()).into());
        }

        let name_desc: Vec<&str> = parts[0].split(" : ").collect();
        if name_desc.len() != 2 {
            return Err(AppError::InvalidListLine(line.to_string()).into());
        }

        let name = name_desc[0].trim_start_matches('-').trim().to_string();
        let description = Self::option_from_str(name_desc[1])?;

        let segments: Vec<&str> = parts[1].split(')').collect();
        if segments.len() != 4 {
            return Err(AppError::InvalidListLine(line.to_string()).into());
        }

        let crate_segment = format!("{})", segments[0].trim());
        let source_segment = format!("{})", segments[1].trim());
        let stats_segment = format!("{})", segments[2].trim());
        let status_segment = segments[3];

        let (_, crate_url) = Self::parse_md_link(&crate_segment)?;
        let (source_type, source_url) = Self::parse_md_link(&source_segment)?;
        let (stars, downloads) = Self::parse_stats(&stats_segment)?;
        let (failed, error_message) = Self::parse_status(status_segment)?;

        Ok(Self {
            name,
            description,
            dependency_kind: dependency_kind.clone(),
            crate_url,
            source_type,
            source_url,
            stats: DependencyStats { stars, downloads },
            failed,
            error_message,
        })
    }

    fn option_from_str<T: FromStr>(s: &str) -> Result<Option<T>>
    where
        <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            trimmed
                .parse::<T>()
                .map(Some)
                .map_err(|e| AppError::InvalidListLine(e.to_string()).into())
        }
    }

    pub fn parse_md_link(s: &str) -> Result<(String, Option<String>)> {
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

    pub fn parse_stats(s: &str) -> Result<(Option<u32>, Option<u32>)> {
        let cleaned = s
            .trim_start_matches(&Self::TRIM_PATTERN)
            .trim_end_matches(&Self::TRIM_PATTERN);

        match cleaned {
            text if text.contains('🌟') && text.contains('📦') => {
                let normalized = text.replace('🌟', "").replace('📦', "|");
                let parts: Vec<&str> = normalized.split('|').collect();
                if parts.len() != 2 {
                    return Err(AppError::InvalidStats(cleaned.to_string()).into());
                }
                let stars = parts[0]
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| AppError::InvalidStats(cleaned.to_string()))?;
                let downloads = parts[1]
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| AppError::InvalidStats(cleaned.to_string()))?;
                Ok((Some(stars), Some(downloads)))
            }
            text if text.contains('🌟') => {
                let parts: Vec<&str> = text.split('🌟').collect();
                let stars = parts[1]
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| AppError::InvalidStats(cleaned.to_string()))?;
                Ok((Some(stars), None))
            }
            text if text.contains('📦') => {
                let parts: Vec<&str> = text.split('📦').collect();
                let downloads = parts[1]
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| AppError::InvalidStats(cleaned.to_string()))?;
                Ok((None, Some(downloads)))
            }
            _ => Ok((None, None)),
        }
    }

    pub fn parse_status(s: &str) -> Result<(bool, Option<String>)> {
        let cleaned = s
            .trim_start_matches(&Self::TRIM_PATTERN)
            .trim_end_matches(&Self::TRIM_PATTERN);

        if cleaned.contains('✅') {
            Ok((false, None))
        } else if cleaned.contains('❌') {
            let parts: Vec<&str> = cleaned.split('❌').collect();
            let message = parts.get(1).map(|msg| msg.trim()).unwrap_or("");
            if message.is_empty() {
                Ok((true, None))
            } else {
                Ok((true, Some(message.to_string())))
            }
        } else {
            Err(AppError::InvalidStatus(cleaned.to_string()).into())
        }
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
    use super::*;

    #[test]
    fn parse_status_variants() {
        let (failed, error) = DependencyInfo::parse_status("✅").unwrap();
        assert!(!failed);
        assert!(error.is_none());

        let (failed, error) =
            DependencyInfo::parse_status("❌ Unknown error: failed to fetch repository info")
                .unwrap();
        assert!(failed);
        assert_eq!(
            error,
            Some("Unknown error: failed to fetch repository info".to_string())
        );
    }

    #[test]
    fn parse_stats_variants() {
        let (stars, downloads) = DependencyInfo::parse_stats("🌟 1000 📦 100").unwrap();
        assert_eq!(stars, Some(1000));
        assert_eq!(downloads, Some(100));

        let (stars, downloads) = DependencyInfo::parse_stats("🌟 1000").unwrap();
        assert_eq!(stars, Some(1000));
        assert!(downloads.is_none());
    }

    #[test]
    fn parse_md_link_cases() {
        let (label, url) =
            DependencyInfo::parse_md_link("[GitHub](https://github.com/serde-rs/serde)").unwrap();
        assert_eq!(label, "GitHub");
        assert_eq!(url, Some("https://github.com/serde-rs/serde".to_string()));
    }
}
