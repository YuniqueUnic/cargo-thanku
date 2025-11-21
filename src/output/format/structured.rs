use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::output::dependency::DependencyInfo;

use super::Formatter;

#[derive(Debug, Serialize, Deserialize)]
struct DependencyList {
    dependencies: Vec<DependencyInfo>,
}

pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        Ok(serde_json::to_string_pretty(deps)?)
    }

    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        Ok(serde_json::from_str(content)?)
    }
}

pub struct TomlFormatter;

impl Formatter for TomlFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        let deps_list = DependencyList {
            dependencies: deps.to_vec(),
        };
        Ok(toml::to_string_pretty(&deps_list)?)
    }

    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        let deps_list: DependencyList = toml::from_str(content)?;
        Ok(deps_list.dependencies)
    }
}

pub struct YamlFormatter;

impl Formatter for YamlFormatter {
    fn format(&self, deps: &[DependencyInfo]) -> Result<String> {
        Ok(serde_yaml::to_string(deps)?)
    }

    fn parse(&self, content: &str) -> Result<Vec<DependencyInfo>> {
        Ok(serde_yaml::from_str(content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::dependency::{DependencyInfo, DependencyKind, DependencyStats};

    fn sample_dependency() -> DependencyInfo {
        DependencyInfo {
            name: "serde".into(),
            description: Some("Serialization".into()),
            dependency_kind: DependencyKind::Normal,
            crate_url: Some("https://crates.io/crates/serde".into()),
            source_type: "GitHub".into(),
            source_url: Some("https://github.com/serde-rs/serde".into()),
            stats: DependencyStats {
                stars: Some(1000),
                downloads: Some(10_000),
            },
            failed: false,
            error_message: None,
        }
    }

    #[test]
    fn toml_roundtrip() {
        let deps = vec![sample_dependency()];
        let formatter = TomlFormatter;
        let content = formatter.format(&deps).unwrap();
        let parsed = formatter.parse(&content).unwrap();
        assert_eq!(parsed.len(), deps.len());
        assert_eq!(parsed[0].name, deps[0].name);
    }

    #[test]
    fn json_roundtrip() {
        let deps = vec![sample_dependency()];
        let formatter = JsonFormatter;
        let content = formatter.format(&deps).unwrap();
        let parsed = formatter.parse(&content).unwrap();
        assert_eq!(parsed[0].name, deps[0].name);
    }

    #[test]
    fn yaml_roundtrip() {
        let deps = vec![sample_dependency()];
        let formatter = YamlFormatter;
        let content = formatter.format(&deps).unwrap();
        let parsed = formatter.parse(&content).unwrap();
        assert_eq!(parsed[0].source_type, deps[0].source_type);
    }
}
