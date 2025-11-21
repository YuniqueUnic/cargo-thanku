use assert_fs::prelude::*;
use cargo_thanku::{output::OutputFormat, travert::Travert};

fn write_temp(content: &str, name: &str) -> assert_fs::NamedTempFile {
    let file = assert_fs::NamedTempFile::new(name).expect("temp file");
    file.write_str(content).expect("write content");
    file
}

#[test]
fn detect_markdown_table_format() {
    let file = write_temp("| Name | Desc |\n|---|---|\n| a | b |", "table.md");
    let travert = Travert::new(file.path()).expect("travert");
    assert_eq!(travert.format, OutputFormat::MarkdownTable);
}

#[test]
fn detect_markdown_list_format() {
    let file = write_temp("- item\n- item2", "list.md");
    let travert = Travert::new(file.path()).expect("travert");
    assert_eq!(travert.format, OutputFormat::MarkdownList);
}

#[test]
fn detect_complex_table_format() {
    let content = "| Left | Right |\n|:---|---:|\n| data | data |";
    let file = write_temp(content, "complex.md");
    let travert = Travert::new(file.path()).expect("travert");
    assert_eq!(travert.format, OutputFormat::MarkdownTable);
}
