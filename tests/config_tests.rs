use cargo_thanku::config::{Config, OutputWriter};

#[test]
fn output_writer_stdout_dash() {
    let mut config = Config::default();
    config.output = Some("-".into());
    match config.get_output_writer().expect("stdout writer") {
        OutputWriter::Stdout(_) => {}
        _ => panic!("expected stdout writer"),
    }
}

#[test]
fn output_writer_file() {
    let temp = assert_fs::NamedTempFile::new("test-output.md").expect("temp file");
    let mut config = Config::default();
    config.output = Some(temp.path().to_path_buf());
    match config.get_output_writer().expect("file writer") {
        OutputWriter::File(_) => {}
        _ => panic!("expected file writer"),
    }
}

#[test]
fn output_writer_default_stdout() {
    let config = Config::default();
    match config.get_output_writer().expect("stdout") {
        OutputWriter::Stdout(_) => {}
        _ => panic!("expected stdout writer"),
    }
}
