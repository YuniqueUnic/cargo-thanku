use anyhow::Result;
use rust_i18n::t;

use crate::{errors::AppError, output::dependency::DependencyInfo};

use super::Formatter;

pub struct CsvFormatter;

impl CsvFormatter {
    fn header() -> impl AsRef<str> {
        t!("output.csv_header").replace('，', ",")
    }

    fn column_num() -> usize {
        CsvFormatter::header().as_ref().split(',').count()
    }
}

impl Formatter for CsvFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let header = CsvFormatter::header();
        let mut output = String::new();
        output.push_str(&format!("\n{}\n", header.as_ref()));

        for dep in deps {
            let (name, description, crates_link, source_link, stats, status) = dep.to_strings();
            let dependency_kind = dep.dependency_kind.to_string();
            let description = description.replace(',', ";");

            output.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                name, description, dependency_kind, crates_link, source_link, stats, status,
            ));
        }

        Ok(output)
    }

    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        let mut lines = content.lines().filter(|line| !line.trim().is_empty());

        let header = lines
            .next()
            .ok_or_else(|| AppError::InvalidCsvContent(content.to_string()))?;

        let columns = header.split(',').collect::<Vec<_>>();
        if columns.len() != CsvFormatter::column_num() {
            return Err(AppError::InvalidCsvContent(content.to_string()).into());
        }

        let mut deps = Vec::new();
        for line in lines {
            deps.push(DependencyInfo::try_from_csv_line(line, columns.len())?);
        }

        Ok(deps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::dependency::{DependencyInfo, DependencyKind, DependencyStats};

    #[test]
    fn csv_roundtrip() {
        let deps = vec![DependencyInfo {
            name: "serde".into(),
            description: Some("Serialization".into()),
            dependency_kind: DependencyKind::Normal,
            crate_url: Some("https://crates.io/crates/serde".into()),
            source_type: "GitHub".into(),
            source_url: Some("https://github.com/serde-rs/serde".into()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: None,
            },
            failed: false,
            error_message: None,
        }];

        let formatter = CsvFormatter;
        let content = formatter.format(&deps).unwrap();
        let parsed = formatter.parse(&content).unwrap();
        assert_eq!(deps[0].name, parsed[0].name);
        assert_eq!(deps[0].stats.stars, parsed[0].stats.stars);
    }
}
