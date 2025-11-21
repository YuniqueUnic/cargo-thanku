use std::path::PathBuf;

use anyhow::Result;
use clap::ArgMatches;
use tracing::{info, instrument};

use crate::{cli::generate_completions, output::OutputFormat, travert::Converter};

#[instrument(skip(matches))]
pub fn completions(matches: &ArgMatches) -> Result<()> {
    if let Some(shell) = matches.get_one::<String>("shell") {
        generate_completions(shell).map_err(|e| {
            anyhow::anyhow!(t!(
                "main.failed_generate_completions",
                error = e.to_string()
            ))
        })?;
    }
    Ok(())
}

#[cfg(debug_assertions)]
#[instrument(skip(matches))]
pub fn test(matches: &ArgMatches) -> Result<()> {
    info!(?matches, "test subcommand invoked");
    println!("{}", t!("app.description"));
    println!("test: {:?}", matches);
    Ok(())
}

#[instrument(skip(matches))]
pub fn convert(matches: &ArgMatches) -> Result<()> {
    let input = matches
        .get_one::<PathBuf>("input")
        .ok_or_else(|| anyhow::anyhow!(t!("main.convert_input_required")))?;

    if !input.is_file() {
        return Err(anyhow::anyhow!(t!("main.convert_input_not_file")));
    }

    let (name, _ext) = input
        .file_name()
        .and_then(|n| n.to_str())
        .map(|_| {
            let stem = input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            let ext = input
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();

            (stem, ext)
        })
        .ok_or_else(|| anyhow::anyhow!(t!("main.convert_invalid_filename")))?;

    let input_dir = input
        .parent()
        .ok_or_else(|| anyhow::anyhow!(t!("main.convert_invalid_filename")))?;

    let output_dir = input_dir.join("converted");
    std::fs::create_dir_all(&output_dir)?;

    let outputs = matches
        .get_many::<String>("outputs")
        .unwrap_or_default()
        .into_iter()
        .map(|format| format.parse::<OutputFormat>().unwrap_or_default())
        .map(|format| {
            let file_name = format!(
                "{}_{}",
                name.trim_end_matches(['_']),
                format.to_identifier()
            );
            output_dir.join(format!(
                "{}.{}",
                file_name.trim_end_matches(['_']),
                format.to_extension()
            ))
        })
        .collect::<Vec<_>>();

    let converter = Converter::new(input, &outputs)?;
    converter.convert()?;

    Ok(())
}
