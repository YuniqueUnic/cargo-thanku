use std::io::Write;

use anyhow::Result;

use super::{
    dependency::DependencyInfo,
    format::{Formatter, OutputFormat},
};

pub struct OutputManager<W: Write> {
    formatter: Box<dyn Formatter>,
    writer: W,
}

impl<W: Write> OutputManager<W> {
    pub fn new(format: OutputFormat, writer: W) -> Self {
        let formatter = <dyn Formatter>::new(format).expect("invalid formatter");
        Self { formatter, writer }
    }

    pub fn write(&mut self, deps: &[DependencyInfo]) -> Result<()> {
        let content = self.formatter.format(deps)?;
        self.writer.write_all(content.as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::dependency::{DependencyInfo, DependencyKind, DependencyStats};

    fn sample_dep() -> DependencyInfo {
        DependencyInfo {
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
        }
    }

    #[test]
    fn write_to_memory() {
        let deps = vec![sample_dep()];
        let mut buffer = Vec::new();
        let mut manager = OutputManager::new(OutputFormat::MarkdownTable, &mut buffer);
        manager.write(&deps).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("serde"));
    }
}
