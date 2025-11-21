use cargo_thanku::cli::build_cli;

#[test]
fn verify_cli() {
    build_cli().debug_assert();
}
