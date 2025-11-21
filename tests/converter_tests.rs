use std::{fs, path::PathBuf};

use assert_fs::prelude::*;
use cargo_thanku::travert::Converter;

fn sample_markdown() -> &'static str {
    "| 名称 | 描述 | 链接 | 来源 | 统计 | 状态 |\n|---|---|---|---|---|---|\n| 🔍 | Normal | | | | |\n| anyhow | desc | [anyhow](https://crates.io/crates/anyhow) | [GitHub](https://github.com/dtolnay/anyhow) | 🌟 1 | ✅ |"
}

#[test]
fn converter_writes_targets() {
    let temp = assert_fs::TempDir::new().expect("temp dir");
    let input = temp.child("input.md");
    input.write_str(sample_markdown()).expect("write md");

    let converted_dir = temp.child("converted");
    converted_dir.create_dir_all().expect("converted dir");
    let output = converted_dir.child("thanks.json");
    let target_path: PathBuf = output.path().to_path_buf();

    let converter = Converter::new(input.path(), [target_path.as_path()]).expect("converter");
    converter.convert().expect("convert");

    assert!(output.path().is_file());
    let content = fs::read_to_string(output.path()).expect("read output");
    assert!(content.contains("anyhow"));
}
