use cargo_thanku::output::{
    Formatter,
    dependency::{DependencyInfo, DependencyKind, DependencyStats},
    format::markdown::{MarkdownListFormatter, MarkdownTableFormatter},
};

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
fn markdown_table_roundtrip() {
    let deps = vec![sample_dep(DependencyKind::Normal)];
    let formatter = MarkdownTableFormatter;
    let content = formatter.format(&deps).unwrap();
    let parsed = formatter.parse(&content).unwrap();
    assert_eq!(parsed[0].name, deps[0].name);
}

#[test]
fn markdown_list_roundtrip() {
    let deps = vec![sample_dep(DependencyKind::Development)];
    let formatter = MarkdownListFormatter;
    let content = formatter.format(&deps).unwrap();
    let parsed = formatter.parse(&content).unwrap();
    assert_eq!(parsed[0].dependency_kind, DependencyKind::Development);
}

#[test]
fn markdown_table_skips_noise() {
    let deps = vec![sample_dep(DependencyKind::Normal)];
    let formatter = MarkdownTableFormatter;
    let mut content = String::from("Not a table\n");
    content.push_str(&formatter.format(&deps).unwrap());
    content.push_str("\nRandom footer");
    let parsed = formatter.parse(&content).unwrap();
    assert_eq!(parsed.len(), 1);
}

#[test]
fn markdown_list_skips_noise() {
    let deps = vec![sample_dep(DependencyKind::Build)];
    let formatter = MarkdownListFormatter;
    let mut content = formatter.format(&deps).unwrap();
    content.push_str("\n> appendix note");
    let parsed = formatter.parse(&content).unwrap();
    assert_eq!(parsed.len(), 1);
}
