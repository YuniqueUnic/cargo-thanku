use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::output::{DependencyInfo, OutputFormat};

#[derive(Debug, Clone)]
pub struct Travert {
    pub path: PathBuf,
    pub format: OutputFormat,
}

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

    /// TODO: 解析内容，并且转化为 DependencyInfo
    pub fn parse(&self) -> Result<DependencyInfo> {
        let content = std::fs::read_to_string(&self.path)?;
        let format = self.format;

        let dependency_info = match format {
            OutputFormat::MarkdownTable => {
                let dependency_info = DependencyInfo::default();
                dependency_info
            }
            _ => anyhow::bail!(t!(
                "travert.failed_to_parse_content",
                path = self.path.display()
            )),
        };

        Ok(dependency_info)
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

        Ok(if table_re.is_match(content) {
            OutputFormat::MarkdownTable
        } else {
            OutputFormat::MarkdownList
        })
    }

    fn judge_format<P: AsRef<Path>>(path: P) -> Result<OutputFormat> {
        let path = path.as_ref();

        if path.is_file() {
            let extension = path.extension().unwrap_or_default();

            return match extension.to_str() {
                Some("md") => {
                    let content = std::fs::read_to_string(path)?;
                    Self::detect_markdown_content_format(&content)
                }
                Some("csv") => Ok(OutputFormat::Csv),
                Some("toml") => Ok(OutputFormat::Toml),
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
    pub target: Vec<Travert>,
}

impl Converter {
    pub fn new<P: AsRef<Path>>(source: P, target: &[P]) -> Result<Self> {
        Ok(Self {
            source: Travert::new(source)?,
            target: target
                .iter()
                .map(|p| Travert::new(p))
                .collect::<Result<Vec<_>>>()?,
        })
    }

    pub fn new_with_format(source: Travert, target: Vec<Travert>) -> Result<Self> {
        Ok(Self { source, target })
    }

    // TODO: 实现转换
    pub fn convert(&self) -> Result<()> {
        let source_content = std::fs::read_to_string(&self.source.path)?;
        let source_format = self.source.format;

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
