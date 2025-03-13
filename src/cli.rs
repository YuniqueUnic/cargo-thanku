use clap::{Arg, ArgAction, Command};

pub fn build_cli() -> Command {
    Command::new("cargo-thanks")
        .about("Give thanks to your Rust dependencies")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("token")
                .long("token")
                .env("GITHUB_TOKEN")
                .required(true)
                .action(ArgAction::Set)
                .help("GitHub authentication token"),
        )
        .subcommand(
            Command::new("completions")
                .about("Generate shell completions")
                .arg(Arg::new("shell").required(true)),
        )
}

// 生成补全脚本示例
fn generate_completions(shell: &str) {
    clap_complete::generate(
        shell.parse().expect("Invalid shell type"),
        &mut cmd,
        "cargo-thanks",
        &mut std::io::stdout(),
    );
}
