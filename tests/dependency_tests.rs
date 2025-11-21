use cargo_thanku::output::dependency::DependencyInfo;

#[test]
fn parse_status_variants() {
    let (failed, error) = DependencyInfo::parse_status("✅").unwrap();
    assert!(!failed);
    assert!(error.is_none());

    let (failed, error) =
        DependencyInfo::parse_status("❌ Unknown error: failed to fetch repository info").unwrap();
    assert!(failed);
    assert_eq!(
        error,
        Some("Unknown error: failed to fetch repository info".to_string())
    );
}

#[test]
fn parse_stats_variants() {
    let (stars, downloads) = DependencyInfo::parse_stats("🌟 1000 📦 100").unwrap();
    assert_eq!(stars, Some(1000));
    assert_eq!(downloads, Some(100));

    let (stars, downloads) = DependencyInfo::parse_stats("🌟 1000").unwrap();
    assert_eq!(stars, Some(1000));
    assert!(downloads.is_none());
}

#[test]
fn parse_md_link_cases() {
    let (label, url) =
        DependencyInfo::parse_md_link("[GitHub](https://github.com/serde-rs/serde)").unwrap();
    assert_eq!(label, "GitHub");
    assert_eq!(url, Some("https://github.com/serde-rs/serde".to_string()));
}
