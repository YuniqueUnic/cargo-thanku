use clap::{Arg, ArgAction, Command};
use clap_complete::Shell;
use rust_i18n::t;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::instrument;

#[instrument(skip_all)]
pub fn build_cli() -> Command {
    Command::new("cargo-thanku")
        .bin_name("cargo-thanku") // 使用 `thanku` 作为命令名，使得 cargo thanku 可以正常工作
        .version(env!("CARGO_PKG_VERSION"))
        .about(format!("{}", t!("cli.about")))
        .args([
            Arg::new("input")
                .short('i')
                .long("input")
                .help(format!("{}", t!("cli.input_help")))
                .global(true)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(PathBuf))
                .default_value("Cargo.toml"),
            Arg::new("output")
                .short('o')
                .long("output")
                .help(format!("{}", t!("cli.output_help")))
                .global(true)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(PathBuf))
                .default_value("thanks.md"),
            Arg::new("name")
                .short('n')
                .long("name")
                .help(format!("{}", t!("cli.name_help")))
                .global(true)
                .value_hint(clap::ValueHint::FilePath)
                .default_value("thanks"),
            Arg::new("format")
                .short('f')
                .long("format")
                .help(format!("{}", t!("cli.format_help")))
                .global(true)
                .value_parser(["markdown-table", "markdown-list", "json", "toml", "yaml"])
                .default_value("markdown-table"),
            Arg::new("source")
                .short('s')
                .long("source")
                .help(format!("{}", t!("cli.source_help")))
                .global(true)
                .value_parser(["github", "crates-io", "link-empty", "other"])
                .default_value("github"),
            Arg::new("token")
                .short('t')
                .long("token")
                .global(true)
                .env("GITHUB_TOKEN")
                .help(format!("{}", t!("cli.token_help")))
                .action(ArgAction::Set),
            Arg::new("crates-token")
                .short('c')
                .long("crates-token")
                .global(true)
                .env("CRATES_TOKEN")
                .help(format!("{}", t!("cli.crates_token_help")))
                .action(ArgAction::Set),
            Arg::new("language")
                .short('l')
                .long("language")
                .help(format!("{}", t!("cli.language_help")))
                .global(true)
                .env("LANG")
                .value_parser(["zh", "en", "ja", "ko", "es", "fr", "de", "it"])
                .default_value("zh"),
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help(format!("{}", t!("cli.verbose_help")))
                .global(true)
                .env("VERBOSE")
                .default_value("false")
                .action(ArgAction::SetTrue),
        ])
        .subcommand(
            Command::new("completions")
                .about(format!("{}", t!("cli.completions_about")))
                .arg(
                    Arg::new("shell")
                        .help(format!("{}", t!("cli.completions_args.shell_help")))
                        .required(true)
                        .value_parser(["bash", "fish", "zsh", "powershell", "elvish"]),
                ),
        )
}

#[instrument]
pub fn generate_completions(shell: &str) -> anyhow::Result<()> {
    let shell = Shell::from_str(shell)
        .map_err(|_| anyhow::anyhow!(t!("cli.invalid_shell_type", shell = shell)))?;
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
