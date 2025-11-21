use std::io::Read;

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

    fn parse_reader(&self, reader: &mut dyn Read) -> Result<Vec<DependencyInfo>> {
        Ok(serde_json::from_reader(reader)?)
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

    fn parse_reader(&self, reader: &mut dyn Read) -> Result<Vec<DependencyInfo>> {
        Ok(serde_yaml::from_reader(reader)?)
    }
}
