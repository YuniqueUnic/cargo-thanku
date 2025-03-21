use clap::{Arg, ArgAction, ArgGroup, Command};
use clap_complete::Shell;
use rust_i18n::t;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::instrument;

use crate::output::OutputFormat;

// Arg::new("language")
//     .short('l')
//     .long("language")
//     .help(format!("{}", t!("cli.language_help")))
//     .global(true)
//     .env("LANG")
//     .value_parser(|s: &str| {
//         let tag = LanguageTag::parse(s)
//             .map_err(|_| format!("Invalid language tag: {}", s))?;
//         let lang = tag.primary_language();
//         match lang {
//             "zh" | "en" | "ja" | "ko" | "es" | "fr" | "de" | "it" => Ok(lang.to_string()),
//             _ => Err(format!("Unsupported language: {}", lang))
//         }
//     })
//     .default_value("zh"),

/// 定义语言解析器
#[allow(unused)]
#[derive(Clone, Debug)]
struct LanguageParser;

impl clap::builder::TypedValueParser for LanguageParser {
    type Value = String;

    #[instrument]
    fn parse_ref(
        &self,
        cmd: &Command,
        arg: Option<&Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let input = value.to_string_lossy().to_lowercase();

        // 解析语言代码
        let lang_code = if input.contains('_') || input.contains('.') {
            // 处理形如 "en_US.UTF-8" 的格式
            input
                .split(['_', '.', '-'])
                .next()
                .unwrap_or("zh")
                .to_string()
        } else {
            input
        };

        // 验证是否是支持的语言
        match lang_code.as_str() {
            "zh" | "en" | "ja" | "ko" | "es" | "fr" | "de" | "it" => Ok(lang_code),
            _ => {
                // 尝试找到最相似的语言代码
                let supported = ["zh", "en", "ja", "ko", "es", "fr", "de", "it"];
                if let Some(similar) = supported
                    .iter()
                    .min_by_key(|&x| strsim::levenshtein(x, &lang_code))
                {
                    Err(clap::Error::raw(
                        clap::error::ErrorKind::InvalidValue,
                        format!(
                            "Invalid language '{}'. Did you mean '{}'?",
                            lang_code, similar
                        ),
                    ))
                } else {
                    Err(clap::Error::raw(
                        clap::error::ErrorKind::InvalidValue,
                        format!("Unsupported language: {}", lang_code),
                    ))
                }
            }
        }
    }
}

/// 定义输出格式解析器
#[allow(unused)]
#[derive(Clone, Debug)]
struct OutputFormatParser;

impl clap::builder::TypedValueParser for OutputFormatParser {
    type Value = OutputFormat;

    #[instrument]
    fn parse_ref(
        &self,
        cmd: &Command,
        arg: Option<&Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let input = value.to_string_lossy().to_lowercase();
        Ok(OutputFormat::from_str(&input).map_err(|_| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                format!("{}", t!("cli.invalid_output_format", format = input)),
            )
        })?)
    }
}

fn build_global_args() -> [Arg; 2] {
    [
        Arg::new("verbose")
            .short('v')
            .long("verbose")
            .help(format!("{}", t!("cli.verbose_help")))
            .global(true)
            .group("global")
            .display_order(99)
            .env("VERBOSE")
            .default_value("false")
            .action(ArgAction::SetTrue),
        Arg::new("language")
            .short('l')
            .long("language")
            .help(format!("{}", t!("cli.language_help")))
            .global(true)
            .group("global")
            .display_order(98)
            // .env("LANG")
            .value_parser(["zh", "en", "ja", "ko", "es", "fr", "de", "it"])
            // .value_parser(LanguageParser) // Assuming LanguageParser is handled later or is simple
            .default_value("zh"),
    ]
}

fn build_thanku_args() -> [Arg; 8] {
    [
        Arg::new("input")
            .short('i')
            .long("input")
            .aliases(["in"])
            .help(format!("{}", t!("cli.input_help")))
            .display_order(0)
            // .global(true)
            .group("thanku")
            .value_hint(clap::ValueHint::FilePath)
            .value_parser(clap::value_parser!(PathBuf))
            .default_value("Cargo.toml"),
        Arg::new("output")
            .short('o')
            .long("output")
            .aliases(["out"])
            .help(format!("{}", t!("cli.output_help")))
            .display_order(1)
            // .global(true)
            .group("thanku")
            .value_hint(clap::ValueHint::FilePath)
            .value_parser(clap::value_parser!(PathBuf))
            .default_value("thanks.md"),
        Arg::new("format")
            .short('f')
            .long("format")
            .aliases(["fmt", "type"])
            .help(format!("{}", t!("cli.format_help")))
            .display_order(2)
            // .global(true)
            .group("thanku")
            // .value_parser(OutputFormatParser)
            .value_parser(["markdown-table", "markdown-list", "csv", "json", "yaml"])
            .default_value("markdown-table"),
        Arg::new("source")
            .short('s')
            .long("source")
            .aliases(["src"])
            .help(format!("{}", t!("cli.source_help")))
            .display_order(3)
            // .global(true)
            .group("thanku")
            .value_parser(["github", "crates-io", "link-empty", "other"])
            .default_value("github"),
        Arg::new("token")
            .short('t')
            .long("token")
            // .global(true)
            .group("thanku")
            .env("GITHUB_TOKEN")
            .help(format!("{}", t!("cli.token_help")))
            .display_order(4)
            .action(ArgAction::Set),
        Arg::new("no-relative-libs")
            .long("no-relative-libs")
            .aliases(["no-rel-libs", "no-rel"])
            // .global(true)
            .group("thanku")
            .help(format!("{}", t!("cli.no_relative_libs_help")))
            .display_order(5)
            .action(ArgAction::SetTrue),
        Arg::new("concurrent")
            .short('j')
            .long("concurrent")
            .aliases(["con", "conc"])
            .help(format!("{}", t!("cli.concurrent_help")))
            .display_order(6)
            // .global(true)
            .group("thanku")
            .value_parser(clap::value_parser!(usize))
            .default_value("5"),
        Arg::new("retries")
            .short('r')
            .long("retries")
            .aliases(["retry"])
            .help(format!("{}", t!("cli.retries_help")))
            .display_order(7)
            // .global(true)
            .group("thanku")
            .value_parser(clap::value_parser!(u32))
            .default_value("3"),
    ]
}

fn build_convert_args() -> [Arg; 2] {
    [
        Arg::new("input")
            .short('i')
            .long("input")
            .aliases(["in"])
            .group("convert")
            .help(format!("{}", t!("cli.convert_input_help")))
            .display_order(0)
            .value_hint(clap::ValueHint::FilePath)
            .value_parser(clap::value_parser!(PathBuf)),
        Arg::new("outputs") // 支持多选，使用逗号分隔
            .short('o')
            .aliases(["out", "output"])
            .long("outputs")
            .group("convert")
            .help(format!("{}", t!("cli.convert_outputs_help")))
            .value_delimiter(',')
            .display_order(1)
            // .value_parser(OutputFormatParser)
            .value_parser(["markdown-table", "markdown-list", "csv", "json", "yaml"]),
    ]
}

pub fn build_cli() -> Command {
    let global_args = build_global_args();
    let thanku_args = build_thanku_args();
    let convert_args = build_convert_args();

    let global_args_ids = global_args
        .iter()
        .map(|arg| arg.get_id())
        .collect::<Vec<_>>();
    let thanku_args_ids = thanku_args
        .iter()
        .map(|arg| arg.get_id())
        .collect::<Vec<_>>();
    let convert_args_ids = convert_args
        .iter()
        .map(|arg| arg.get_id())
        .collect::<Vec<_>>();

    let global_group = ArgGroup::new("global").args(global_args_ids).multiple(true);
    let thanku_group = ArgGroup::new("thanku").args(thanku_args_ids).multiple(true);
    let convert_group = ArgGroup::new("convert")
        .args(convert_args_ids)
        .multiple(true)
        .required(true);

    let mut cmd = Command::new("cargo-thanku") // Use "cargo-thanku" as the command name for `cargo thanku`
        .bin_name("cargo-thanku") // This tells cargo how to invoke it
        .aliases(["thx", "thxu"])
        .groups([&thanku_group, &global_group])
        .version(env!("CARGO_PKG_VERSION"))
        .about(format!("{}", t!("cli.about")))
        .args(&global_args)
        .args(&thanku_args)
        .subcommands([
            Command::new("thanku")
                .aliases(["thx", "thxu"])
                .group(&thanku_group)
                .about(format!("{}", t!("cli.thanku_about")))
                .hide(true)
                .args(&thanku_args),
            Command::new("convert")
                .group(&convert_group)
                .aliases(["cvt", "conv", "convt"])
                .about(format!("{}", t!("cli.convert_help")))
                .args(&convert_args),
            Command::new("completions")
                .aliases(["comp", "completion"])
                .about(format!("{}", t!("cli.completions_about")))
                .arg(
                    Arg::new("shell")
                        .help(format!("{}", t!("cli.completions_args.shell_help")))
                        .required(true)
                        .value_parser(["bash", "fish", "zsh", "powershell", "elvish"]),
                ),
        ]);

    #[cfg(debug_assertions)]
    {
        cmd = cmd.subcommand(
            Command::new("test")
                .about("test")
                .args(&global_args)
                // .args(&thanku_args)
                .args(&convert_args)
                // .group(&convert_group)
                // .group(&thanku_group)
                .group(&global_group),
        );
    }

    cmd
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
