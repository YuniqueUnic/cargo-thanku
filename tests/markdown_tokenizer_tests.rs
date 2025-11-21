use cargo_thanku::output::dependency::DependencyKind;
use cargo_thanku::output::markdown::tokenizer::{
    ListEntry, MarkdownListTokenizer, MarkdownSection, section_kind_from_header,
};

#[test]
fn parse_list_entry_segments() {
    let line = "- serde : desc - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 42 📦 10) ✅";
    let entry = ListEntry::from_line(line).unwrap();
    assert_eq!(entry.name, "serde");
    assert_eq!(entry.description, Some("desc"));
    assert!(entry.crate_segment.starts_with("[serde]"));
    assert!(entry.status_segment.contains("✅"));
}

#[test]
fn tokenizer_detects_headers_and_items() {
    let content = "# Dependencies\n## Normal\n- serde : desc - [serde](https://crates.io/crates/serde) [GitHub](https://github.com/serde-rs/serde) (🌟 42) ✅";
    let mut saw_header = false;
    let mut saw_item = false;

    for token in MarkdownListTokenizer::new(content) {
        match token.unwrap() {
            MarkdownSection::Header(line) => {
                if section_kind_from_header(line) == Some(DependencyKind::Normal) {
                    saw_header = true;
                }
            }
            MarkdownSection::Item(entry) => {
                saw_item = true;
                assert_eq!(entry.name, "serde");
            }
        }
    }

    assert!(saw_header && saw_item);
}
