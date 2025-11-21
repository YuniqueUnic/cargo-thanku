use anyhow::Result;
use regex::Regex;
use rust_i18n::t;
use tracing::warn;

use crate::output::dependency::{DependencyInfo, DependencyKind};

use super::Formatter;

pub struct MarkdownTableFormatter;

impl MarkdownTableFormatter {
    pub(crate) fn column_count() -> usize {
        MarkdownTableFormatter::header().as_ref().split('|').count() - 2
    }

    fn header() -> impl AsRef<str> {
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

    fn separator() -> impl AsRef<str> {
        let column_num = MarkdownTableFormatter::column_count();
        format!("|{}", "---|".repeat(column_num))
    }

    fn take_sort_dependencies<'a>(
        deps: &'a [DependencyInfo],
        kind: &DependencyKind,
    ) -> Vec<&'a DependencyInfo> {
        let mut filtered = deps
            .iter()
            .filter(|dep| dep.dependency_kind == *kind)
            .collect::<Vec<_>>();
        filtered.sort_by(|a, b| a.name.cmp(&b.name));
        filtered
    }

    fn first_table(content: &str) -> Option<&str> {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < 2 {
            return None;
        }

        for i in 0..lines.len() - 1 {
            let header_line = lines[i].trim();
            if !header_line.starts_with('|') && !header_line.contains('|') {
                continue;
            }

            let separator_line = lines[i + 1].trim();
            if !Self::is_valid_separator(separator_line) {
                continue;
            }

            let header_columns = Self::count_columns(header_line);
            if header_columns != Self::count_columns(separator_line) {
                continue;
            }

            let mut end_idx = i + 2;
            while end_idx < lines.len() {
                let row = lines[end_idx].trim();
                if row.is_empty() || (!row.starts_with('|') && !row.contains('|')) {
                    break;
                }
                if Self::count_columns(row) != header_columns {
                    break;
                }
                end_idx += 1;
            }

            if end_idx >= i + 2 {
                let start_pos = content.find(lines[i])?;
                let end_line_start = content.find(lines[end_idx - 1])?;
                let end_pos = end_line_start + lines[end_idx - 1].len();
                return Some(&content[start_pos..end_pos]);
            }
        }

        None
    }

    fn is_valid_separator(line: &str) -> bool {
        if !line.contains('|') {
            return false;
        }

        for cell in Self::split_row(line) {
            let trimmed = cell.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !trimmed.chars().all(|c| c == '-' || c == ':' || c == ' ') {
                return false;
            }
            if !trimmed.contains('-') {
                return false;
            }
        }

        true
    }

    fn count_columns(line: &str) -> usize {
        Self::split_row(line).len()
    }

    fn split_row(line: &str) -> Vec<&str> {
        let trimmed = line.trim();
        let processed = if trimmed.starts_with('|') && trimmed.ends_with('|') {
            &trimmed[1..trimmed.len() - 1]
        } else if trimmed.starts_with('|') {
            &trimmed[1..]
        } else if trimmed.ends_with('|') {
            &trimmed[..trimmed.len() - 1]
        } else {
            trimmed
        };

        processed.split('|').collect()
    }
}

impl Formatter for MarkdownTableFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let mut output = String::new();
        output.push_str(&format!("\n{}\n", Self::header().as_ref()));
        output.push_str(&format!("{}\n", Self::separator().as_ref()));

        for kind in DependencyKind::ordered() {
            let mut show_header = true;
            let deps = Self::take_sort_dependencies(deps, &kind);
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
        let Some(table) = Self::first_table(content) else {
            return Ok(vec![]);
        };

        let mut deps = Vec::new();
        let mut dependency_kind = DependencyKind::Unknown;
        for line in table.lines().skip(2) {
            let trimmed = line.trim();
            if trimmed.contains(DependencyKind::Normal.to_md_table_header().as_ref()) {
                dependency_kind = DependencyKind::Normal;
                continue;
            } else if trimmed.contains(DependencyKind::Development.to_md_table_header().as_ref()) {
                dependency_kind = DependencyKind::Development;
                continue;
            } else if trimmed.contains(DependencyKind::Build.to_md_table_header().as_ref()) {
                dependency_kind = DependencyKind::Build;
                continue;
            } else if trimmed.contains(DependencyKind::Unknown.to_md_table_header().as_ref()) {
                dependency_kind = DependencyKind::Unknown;
                continue;
            }

            let dep = DependencyInfo::try_from_md_table_line(trimmed, &dependency_kind)?;
            deps.push(dep);
        }

        Ok(deps)
    }
}

pub struct MarkdownListFormatter;

impl MarkdownListFormatter {
    fn header() -> impl AsRef<str> {
        format!("# {}", t!("output.dependencies"))
    }

    fn first_list(content: &str) -> Option<&str> {
        let regex = Regex::new(r"(?m)^(#|##) .+$").ok()?;
        let headers: Vec<_> = regex.find_iter(content).collect();

        if headers.len() < 2 {
            warn!(
                "{}",
                t!("output.invalid_list_header_num", num = headers.len())
            );
            return None;
        }

        let start_idx = headers
            .iter()
            .position(|m| content[m.start()..].starts_with("# "))?;
        let start_header = headers[start_idx];
        let start_pos = start_header.start();

        let end_pos = headers
            .iter()
            .skip(start_idx + 1)
            .find(|m| content[m.start()..].starts_with("# "))
            .map(|m| m.start())
            .unwrap_or_else(|| {
                content[start_pos..]
                    .find("\n### ")
                    .map(|pos| start_pos + pos)
                    .unwrap_or_else(|| content.len())
            });

        let list_content = &content[start_pos..end_pos];
        let lines_after_header = list_content
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();

        if lines_after_header
            .iter()
            .any(|line| line.starts_with("## "))
            && lines_after_header.iter().any(|line| {
                DependencyInfo::try_from_md_list_line(line, &DependencyKind::Unknown).is_ok()
            })
        {
            Some(list_content)
        } else {
            warn!("{}", t!("output.no_valid_list_items_found"));
            None
        }
    }
}

impl Formatter for MarkdownListFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let mut output = String::new();
        output.push_str(&format!("\n{}\n", Self::header().as_ref()));

        for kind in DependencyKind::ordered() {
            let mut show_header = true;
            let deps = MarkdownTableFormatter::take_sort_dependencies(deps, &kind);
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
        let Some(list) = Self::first_list(content) else {
            return Ok(vec![]);
        };

        let mut deps = Vec::new();
        let mut dependency_kind = DependencyKind::Unknown;
        for line in list.lines() {
            let trimmed = line.trim();
            if trimmed.contains(DependencyKind::Normal.to_md_list_header().as_ref()) {
                dependency_kind = DependencyKind::Normal;
                continue;
            } else if trimmed.contains(DependencyKind::Development.to_md_list_header().as_ref()) {
                dependency_kind = DependencyKind::Development;
                continue;
            } else if trimmed.contains(DependencyKind::Build.to_md_list_header().as_ref()) {
                dependency_kind = DependencyKind::Build;
                continue;
            } else if trimmed.contains(DependencyKind::Unknown.to_md_list_header().as_ref()) {
                dependency_kind = DependencyKind::Unknown;
                continue;
            }

            match DependencyInfo::try_from_md_list_line(trimmed, &dependency_kind) {
                Ok(dep) => deps.push(dep),
                Err(_) => warn!("{}", t!("output.failed_to_parse_list_line", line = trimmed)),
            }
        }

        Ok(deps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::dependency::{DependencyInfo, DependencyKind, DependencyStats};

    fn sample_dep(kind: DependencyKind) -> DependencyInfo {
        DependencyInfo {
            name: format!("dep-{kind:?}"),
            description: Some("desc".into()),
            dependency_kind: kind,
            crate_url: Some("https://crates.io/crates/sample".into()),
            source_type: "GitHub".into(),
            source_url: Some("https://github.com/example/repo".into()),
            stats: DependencyStats {
                stars: Some(42),
                downloads: Some(100),
            },
            failed: false,
            error_message: None,
        }
    }

    #[test]
    fn table_roundtrip() {
        let deps = vec![sample_dep(DependencyKind::Normal)];
        let formatter = MarkdownTableFormatter;
        let content = formatter.format(&deps).unwrap();
        let parsed = formatter.parse(&content).unwrap();
        assert_eq!(parsed[0].name, deps[0].name);
    }

    #[test]
    fn list_roundtrip() {
        let deps = vec![sample_dep(DependencyKind::Development)];
        let formatter = MarkdownListFormatter;
        let content = formatter.format(&deps).unwrap();
        let parsed = formatter.parse(&content).unwrap();
        assert_eq!(parsed[0].dependency_kind, DependencyKind::Development);
    }

    #[test]
    fn detect_table_section() {
        const TABLE: &str = r"hfshdfhsfjsdjfgg
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

        assert!(MarkdownTableFormatter::first_table(TABLE).is_some());
    }

    #[test]
    fn detect_list_section() {
        const LIST: &str = "# 依赖项\n## Normal\n- dep : desc - [dep](https://crates.io/crates/dep) [GitHub](https://github.com/x/y) (🌟 100 📦 100) ✅";
        assert!(MarkdownListFormatter::first_list(LIST).is_some());
    }
}
