use std::io::BufRead;

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
        let mut cursor = std::io::Cursor::new(content.as_bytes());
        self.parse_reader(&mut cursor)
    }

    fn parse_reader(&self, reader: &mut dyn BufRead) -> Result<Vec<DependencyInfo>> {
        let mut header = String::new();
        loop {
            header.clear();
            if reader.read_line(&mut header)? == 0 {
                return Ok(vec![]);
            }
            if !header.trim().is_empty() {
                break;
            }
        }
        let header = header.trim();
        let columns = header.split(',').collect::<Vec<_>>();
        if columns.len() != CsvFormatter::column_num() {
            return Err(AppError::InvalidCsvContent(header.to_string()).into());
        }

        let mut deps = Vec::new();
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line)?;
            if bytes == 0 {
                break;
            }
            if line.trim().is_empty() {
                continue;
            }
            let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
            deps.push(DependencyInfo::try_from_csv_line(trimmed, columns.len())?);
        }

        Ok(deps)
    }
}
