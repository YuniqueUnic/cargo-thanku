use clap::{Arg, ArgAction, Command};
use clap_complete::Shell;
use std::path::PathBuf;
use std::str::FromStr;

pub fn build_cli() -> Command {
    Command::new("cargo-thanku")
        .bin_name("cargo-thanku")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Generate acknowledgments for your Rust project dependencies")
        .args([
            Arg::new("input")
                .short('i')
                .long("input")
                .help("Input Cargo.toml file or project directory")
                .value_parser(clap::value_parser!(PathBuf))
                .default_value("Cargo.toml"),
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output file path")
                .value_parser(clap::value_parser!(PathBuf))
                .default_value("thanks.md"),
            Arg::new("name")
                .short('n')
                .long("name")
                .help("Output file name without extension")
                .default_value("thanks"),
            Arg::new("type")
                .short('t')
                .long("type")
                .help("Output format type")
                .value_parser(["markdown-table", "markdown-list", "json", "toml", "yaml"])
                .default_value("markdown-table"),
            Arg::new("link")
                .short('l')
                .long("link")
                .help("Source of dependency links")
                .value_parser(["github", "crates-io", "link-empty", "other"])
                .default_value("github"),
            Arg::new("token")
                .long("token")
                .env("GITHUB_TOKEN")
                .help("GitHub authentication token (required for GitHub operations)")
                .action(ArgAction::Set),
        ])
        .subcommand(
            Command::new("completions")
                .about("Generate shell completions")
                .arg(
                    Arg::new("shell")
                        .help("Target shell")
                        .required(true)
                        .value_parser(["bash", "fish", "zsh", "powershell"]),
                ),
        )
}

pub fn generate_completions(shell: &str) -> Result<(), Box<dyn std::error::Error>> {
    let shell = Shell::from_str(shell).map_err(|_| format!("Invalid shell type: {}", shell))?;
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
