use anyhow::Result;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

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

    fn detect_markdown_format_by_name(path: &Path) -> Result<OutputFormat> {
        let file_name = path.file_stem();

        if file_name.is_none() {
            return Ok(OutputFormat::MarkdownList);
        }

        let file_name = file_name.unwrap().to_string_lossy().to_string();
        let format_identifier = file_name.split(['_']).next_back();

        match format_identifier {
            Some("md") => Ok(OutputFormat::MarkdownTable),
            Some("mt") => Ok(OutputFormat::MarkdownList),
            _ => Ok(OutputFormat::MarkdownList),
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
        if let Some(captures) = table_re.captures(content)
            && captures.len() >= 3
                && let (Some(header_match), Some(separator_match)) =
                    (captures.get(1), captures.get(2))
                {
                    let header_line = header_match.as_str().trim();
                    let separator_line = separator_match.as_str().trim();

                    let header_parts: Vec<&str> = header_line.split('|').collect();

                    let separator_parts: Vec<&str> = separator_line.split('|').collect();

                    if !header_parts.is_empty() && header_parts.len() == separator_parts.len() {
                        return Ok(OutputFormat::MarkdownTable);
                    }
                }

        Ok(OutputFormat::MarkdownList)
    }

    fn judge_format<P: AsRef<Path>>(path: P) -> Result<OutputFormat> {
        let path = path.as_ref();

        let file_exists = path.exists();
        let extension = path.extension().unwrap_or_default().to_ascii_lowercase();
        // println!("extension:  {}", &extension.clone().into_string().unwrap());

        match extension.to_str() {
            Some("md") => {
                if file_exists {
                    let content = std::fs::read_to_string(path)?;
                    Self::detect_markdown_content_format(&content)
                } else {
                    Self::detect_markdown_format_by_name(path)
                }
            }
            Some("csv") => Ok(OutputFormat::Csv),
            Some("toml") => Ok(OutputFormat::Toml),
            Some("yml") => Ok(OutputFormat::Yaml),
            Some("yaml") => Ok(OutputFormat::Yaml),
            Some("json") => Ok(OutputFormat::Json),
            _ => anyhow::bail!(t!("travert.failed_to_judge_format", path = path.display())),
        }

        // anyhow::bail!(t!("travert.failed_to_judge_format", path = path.display()))
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
        let mut reader = BufReader::new(File::open(&self.source.path)?);
        let formatter = <dyn output::Formatter>::new(self.source.format)?;
        let dependencies_info = formatter.parse_reader(&mut reader)?;

        for target in &self.targets {
            let file = std::fs::File::create(&target.path)?;
            let mut writer = BufWriter::new(file);
            let mut manager = output::OutputManager::new(target.format, &mut writer);
            manager.write(&dependencies_info)?;
            writer.flush()?;
            println!(
                "{}",
                t!("travert.write_success", path = target.path.display())
            );
        }

        Ok(())
    }
}
