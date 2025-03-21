use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::output::{self, OutputFormat};

#[derive(Debug, Clone)]
pub struct Travert {
    pub path: PathBuf,
    pub format: OutputFormat,
}

#[allow(dead_code)]
impl Travert {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            format: Self::judge_format(path)?,
        })
    }

    pub fn new_with_format<P: AsRef<Path>>(path: P, format: OutputFormat) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            format,
        }
    }

    /// 判断是 markdown 表格还是 markdown 列表
    fn detect_markdown_content_format(content: &str) -> Result<OutputFormat> {
        let table_re = regex::Regex::new(
            r"(?x)(?m)
            ^\s*\|
            ([^|\n]+(?:\s*\|\s*[^|\n]+)*)
            \|\s*$\n
            ^\s*\|
            ([-:]+(?:\s*\|\s*[-:]+)*)
            \|\s*$",
        )?;

        // find the first match
        // then split it into groups
        if let Some(captures) = table_re.captures(content) {
            if captures.len() >= 3 {
                if let (Some(header_match), Some(separator_match)) =
                    (captures.get(1), captures.get(2))
                {
                    let header_line = header_match.as_str().trim();
                    let separator_line = separator_match.as_str().trim();

                    let header_parts: Vec<&str> = header_line.split('|').collect();

                    let separator_parts: Vec<&str> = separator_line.split('|').collect();

                    if header_parts.len() > 0 && header_parts.len() == separator_parts.len() {
                        return Ok(OutputFormat::MarkdownTable);
                    }
                }
            }
        }

        Ok(OutputFormat::MarkdownList)
    }

    fn judge_format<P: AsRef<Path>>(path: P) -> Result<OutputFormat> {
        let path = path.as_ref();

        if path.is_file() {
            let extension = path.extension().unwrap_or_default().to_ascii_lowercase();

            return match extension.to_str() {
                Some("md") => {
                    let content = std::fs::read_to_string(path)?;
                    Self::detect_markdown_content_format(&content)
                }
                Some("csv") => Ok(OutputFormat::Csv),
                // Some("toml") => Ok(OutputFormat::Toml),
                Some("yml") => Ok(OutputFormat::Yaml),
                Some("yaml") => Ok(OutputFormat::Yaml),
                Some("json") => Ok(OutputFormat::Json),
                _ => anyhow::bail!(t!("travert.failed_to_judge_format", path = path.display())),
            };
        }

        anyhow::bail!(t!("travert.failed_to_judge_format", path = path.display()))
    }
}

#[derive(Debug, Clone)]
pub struct Converter {
    pub source: Travert,
    pub targets: Vec<Travert>,
}

impl Converter {
    pub fn new<P: AsRef<Path>>(source: P, targets: impl IntoIterator<Item = P>) -> Result<Self> {
        Ok(Self {
            source: Travert::new(source)?,
            targets: targets
                .into_iter()
                .map(|p| Travert::new(p))
                .collect::<Result<Vec<_>>>()?,
        })
    }

    #[allow(dead_code)]
    pub fn new_with_format(source: Travert, target: Vec<Travert>) -> Result<Self> {
        Ok(Self {
            source,
            targets: target,
        })
    }

    pub fn convert(&self) -> Result<()> {
        let source_content = std::fs::read_to_string(&self.source.path)?;

        let formatter = <dyn output::Formatter>::new(self.source.format)?;
        let dependencies_info = formatter.parse(&source_content)?;

        // 预生成所有目标格式内容
        let outputs: Result<Vec<_>> = self
            .targets
            .iter()
            .map(|target| {
                let mut buffer = Vec::new();
                let mut manager = output::OutputManager::new(target.format, &mut buffer);
                manager.write(&dependencies_info)?;
                Ok((&target.path, buffer))
            })
            .collect();

        // 批量写入（减少系统调用次数）
        for (path, data) in outputs? {
            std::fs::write(path, data)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_markdown_table() -> Result<()> {
        let content = "\
        | 名称 | 描述 |\n\
        |---|---|\n\
        | hello | world |";
        let format = Travert::detect_markdown_content_format(&content)?;
        assert_eq!(format, OutputFormat::MarkdownTable);
        Ok(())
    }

    #[test]
    fn test_complex_table() -> Result<()> {
        // 含对齐符号的表格
        let content1 = "\
    | Left | Center | Right |\n\
    |:-----|:------:|------:|\n\
    | data | data   | data  |";
        assert_eq!(
            Travert::detect_markdown_content_format(&content1)?,
            OutputFormat::MarkdownTable
        );

        // 含空格的列
        let content2 = "\
    | Column 1   | Column 2 |\n\
    |------------|----------|\n\
    | some value | another  |";
        assert_eq!(
            Travert::detect_markdown_content_format(&content2)?,
            OutputFormat::MarkdownTable
        );

        // 伪表格（列表项）
        let content3 = "\
    - Item 1\n\
    - Item 2";
        assert_eq!(
            Travert::detect_markdown_content_format(&content3)?,
            OutputFormat::MarkdownList
        );

        Ok(())
    }

    #[test]
    fn test_detect_markdown_list() -> Result<()> {
        let content = "
        * hello
        * world
        ";
        let format = Travert::detect_markdown_content_format(&content)?;
        assert_eq!(format, OutputFormat::MarkdownList);
        Ok(())
    }
    #[test]
    fn test_detect_markdown_list_2() -> Result<()> {
        let content = "
        - | 名称 | 描述 |
        - |:---:|:---:|
        - | hello | world |
        ";
        let format = Travert::detect_markdown_content_format(&content)?;
        assert_eq!(format, OutputFormat::MarkdownList);
        Ok(())
    }
}
