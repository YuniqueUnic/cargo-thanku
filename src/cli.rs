use clap::{Arg, ArgAction, Command};
use clap_complete::Shell;
use rust_i18n::t;
use std::path::PathBuf;
use std::str::FromStr;

pub fn build_cli() -> Command {
    Command::new("cargo-thanku")
        .bin_name("cargo-thanku") // 使用 `thanku` 作为命令名，使得 cargo thanku 可以正常工作
        .version(env!("CARGO_PKG_VERSION"))
        .about(format!("{}", t!("cli.about.zh")))
        .args([
            Arg::new("input")
                .short('i')
                .long("input")
                .help(format!("{}", t!("cli.input_help.zh")))
                .value_parser(clap::value_parser!(PathBuf))
                .default_value("Cargo.toml"),
            Arg::new("output")
                .short('o')
                .long("output")
                .help(format!("{}", t!("cli.output_help.zh")))
                .value_parser(clap::value_parser!(PathBuf))
                .default_value("thanks.md"),
            Arg::new("name")
                .short('n')
                .long("name")
                .help(format!("{}", t!("cli.name_help.zh")))
                .default_value("thanks"),
            Arg::new("format")
                .short('f')
                .long("format")
                .help(format!("{}", t!("cli.format_help.zh")))
                .value_parser(["markdown-table", "markdown-list", "json", "toml", "yaml"])
                .default_value("markdown-table"),
            Arg::new("source")
                .short('s')
                .long("source")
                .help(format!("{}", t!("cli.source_help.zh")))
                .value_parser(["github", "crates-io", "link-empty", "other"])
                .default_value("github"),
            Arg::new("token")
                .short('t')
                .long("token")
                .env("GITHUB_TOKEN")
                .help(format!("{}", t!("cli.token_help.zh")))
                .action(ArgAction::Set),
            Arg::new("crates-token")
                .short('c')
                .long("crates-token")
                .env("CRATES_TOKEN")
                .help(format!("{}", t!("cli.crates_token_help.zh")))
                .action(ArgAction::Set),
            Arg::new("language")
                .short('l')
                .long("language")
                .help(format!("{}", t!("cli.language_help.zh")))
                .value_parser(["zh", "en", "ja", "ko", "es", "fr", "de", "it"])
                .default_value("zh"),
        ])
        .subcommand(
            Command::new("completions")
                .about(format!("{}", t!("cli.completions_about.zh")))
                .arg(
                    Arg::new("shell")
                        .help(format!("{}", t!("cli.completions_args.shell_help.zh")))
                        .required(true)
                        .value_parser(["bash", "fish", "zsh", "powershell"]),
                ),
        )
}

pub fn generate_completions(shell: &str) -> Result<(), Box<dyn std::error::Error>> {
    let shell = Shell::from_str(shell)
        .map_err(|_| format!("{}", t!("cli.invalid_shell_type.zh", shell = shell)))?;
    let mut cmd = build_cli();
    clap_complete::generate(shell, &mut cmd, "cargo-thanku", &mut std::io::stdout());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        build_cli().debug_assert();
    }
}
