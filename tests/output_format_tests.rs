use cargo_thanku::output::{
    dependency::{DependencyInfo, DependencyKind, DependencyStats},
    format::{CsvFormatter, JsonFormatter, TomlFormatter, YamlFormatter},
    Formatter, OutputFormat, OutputManager,
};

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
            downloads: Some(10_000),
        },
        failed: false,
        error_message: None,
    }
}

#[test]
fn csv_roundtrip() {
    let deps = vec![sample_dep()];
    let formatter = CsvFormatter;
    let content = formatter.format(&deps).unwrap();
    let parsed = formatter.parse(&content).unwrap();
    assert_eq!(parsed[0].name, "serde");
}

#[test]
fn toml_roundtrip() {
    let deps = vec![sample_dep()];
    let formatter = TomlFormatter;
    let content = formatter.format(&deps).unwrap();
    let parsed = formatter.parse(&content).unwrap();
    assert_eq!(parsed.len(), 1);
}

#[test]
fn json_roundtrip() {
    let deps = vec![sample_dep()];
    let formatter = JsonFormatter;
    let content = formatter.format(&deps).unwrap();
    let parsed = formatter.parse(&content).unwrap();
    assert_eq!(parsed[0].source_type, "GitHub");
}

#[test]
fn yaml_roundtrip() {
    let deps = vec![sample_dep()];
    let formatter = YamlFormatter;
    let content = formatter.format(&deps).unwrap();
    let parsed = formatter.parse(&content).unwrap();
    assert_eq!(
        parsed[0].crate_url.as_deref(),
        Some("https://crates.io/crates/serde")
    );
}

#[test]
fn output_manager_writes_to_buffer() {
    let deps = vec![sample_dep()];
    let mut buffer = Vec::new();
    let mut manager = OutputManager::new(OutputFormat::MarkdownTable, &mut buffer);
    manager.write(&deps).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("serde"));
}
