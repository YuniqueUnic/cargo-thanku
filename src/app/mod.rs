use anyhow::Result;
use tracing::Level;

use crate::{cli::build_cli, config::Config};

pub mod commands;
mod logging;
pub mod pipeline;

pub async fn run() -> Result<()> {
    let cli = build_cli();
    let matches = cli.get_matches();

    let language = matches
        .get_one::<String>("language")
        .cloned()
        .unwrap_or_else(|| "zh".to_string());
    let verbose = matches.get_flag("verbose");

    rust_i18n::set_locale(&language);
    logging::init(if verbose { Level::DEBUG } else { Level::INFO })?;

    let config = Config::from_matches(&matches)?;
    Config::init(config)?;

    if let Some(matches) = matches.subcommand_matches("completions") {
        return commands::completions(matches);
    }

    #[cfg(debug_assertions)]
    {
        if let Some(matches) = matches.subcommand_matches("test") {
            return commands::test(matches);
        }
    }

    if let Some(matches) = matches.subcommand_matches("convert") {
        return commands::convert(matches);
    }

    pipeline::process_dependencies().await
}
