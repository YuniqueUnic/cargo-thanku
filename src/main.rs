mod cli;
mod config;
mod errors;
mod sources;

use anyhow::Result;
use cargo_metadata::MetadataCommand;
use rust_i18n::t;
use std::collections::HashMap;
use tracing::{debug, info, instrument};

use crate::{
    cli::{build_cli, generate_completions},
    config::{Config, OutputFormat},
    errors::AppError,
    sources::{CratesioClient, GitHubClient, Source},
};

#[macro_use]
extern crate rust_i18n;

rust_i18n::i18n!(
    "locales",
    fallback = ["zh", "en", "ja", "ko", "es", "fr", "de", "it"]
);

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    init_log(tracing::Level::INFO)?;

    let matches = build_cli().get_matches();

    // Handle completions subcommand
    if let Some(matches) = matches.subcommand_matches("completions") {
        if let Some(shell) = matches.get_one::<String>("shell") {
            generate_completions(shell).map_err(|e| {
                anyhow::anyhow!(t!(
                    "main.failed_generate_completions",
                    error = e.to_string()
                ))
            })?;
            return Ok(());
        }
    }

    // Initialize global config
    let config = Config::from_matches(&matches)?;
    Config::init(config)?;

    process_dependencies().await
}

#[instrument]
pub fn init_log(log_level: tracing::Level) -> Result<()> {
    // 初始化日志
    let mut log_fmt = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(log_level.into())
                .from_env_lossy(),
        )
        .with_level(true);

    #[cfg(debug_assertions)]
    {
        log_fmt = log_fmt
            .with_target(true)
            .with_thread_ids(true)
            .with_line_number(true)
            .with_file(true);
    }

    log_fmt.init();
    Ok(())
}

#[instrument(skip_all)]
async fn process_dependencies() -> Result<()> {
    let config = Config::global()?;

    // Get cargo metadata
    let metadata = MetadataCommand::new()
        .manifest_path(&config.input)
        .exec()
        .map_err(AppError::MetadataError)?;

    // Collect all dependencies
    let mut deps = HashMap::new();
    for pkg in &metadata.packages {
        for dep in &pkg.dependencies {
            deps.entry(dep.name.clone()).or_insert_with(|| dep.clone());
        }
    }

    debug!("{}", t!("main.found_dependencies", count = deps.len()));

    // Initialize clients
    let crates_io_client = CratesioClient::new();
    let github_client = if let Some(token) = &config.github_token {
        Some(GitHubClient::new(token)?)
    } else {
        None
    };

    // Process each dependency
    let mut results = Vec::new();
    for (name, _) in deps {
        match process_dependency(&name, &crates_io_client, github_client.as_ref()).await {
            Ok(source) => {
                results.push((name, source));
            }
            Err(e) => {
                tracing::warn!(
                    "{}",
                    t!(
                        "main.failed_to_process_dependency",
                        name = name,
                        error = e.to_string()
                    )
                );
            }
        }
    }

    // Generate output
    generate_output(&results, &config.format)?;

    Ok(())
}

#[instrument(skip(crates_io_client, github_client))]
async fn process_dependency(
    name: &str,
    crates_io_client: &CratesioClient,
    github_client: Option<&GitHubClient>,
) -> Result<Source> {
    // Get crate info from crates.io
    let crate_info = crates_io_client.get_crate_info(name).await?;

    // Try to get source information
    if let Some(repo_url) = &crate_info.repository {
        if let Some(mut source) = Source::from_url(&Some(repo_url.clone())) {
            // If it's a GitHub repository and we have a token, try to star it and get more info
            if let (Some(client), Source::GitHub { owner, repo, stars }) =
                (github_client, &mut source)
            {
                if let Ok(repo_info) = client.get_repository_info(&owner, &repo).await {
                    *stars = Some(repo_info.stargazers_count);
                    client.star_repository(&owner, &repo).await?;
                    info!(
                        "💖 {} {}",
                        name,
                        yansi::Paint::new(repo_info.html_url).dim()
                    );
                }
            }
            Ok(source)
        } else {
            Ok(Source::Other {
                description: format!("{}", t!("main.unknown_source", source = repo_url)),
            })
        }
    } else {
        Ok(Source::CratesIo {
            name: name.to_string(),
            downloads: Some(crate_info.downloads),
        })
    }
}

#[instrument(skip(results))]
fn generate_output(results: &[(String, Source)], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::MarkdownTable => {
            println!(
                "| {} | {} | {} |",
                t!("main.name"),
                t!("main.source"),
                t!("main.stats")
            );
            println!("|------|--------|-------|");
            for (name, source) in results {
                match source {
                    Source::GitHub { owner, repo, stars } => {
                        println!(
                            "| {} | [GitHub](https://github.com/{}/{}) | 🌟 {} |",
                            name,
                            owner,
                            repo,
                            stars.unwrap_or(0)
                        );
                    }
                    Source::CratesIo { downloads, .. } => {
                        println!(
                            "| {} | [crates.io](https://crates.io/crates/{}) | 📦 {} |",
                            name,
                            name,
                            downloads.unwrap_or(0)
                        );
                    }
                    Source::Link { url } => {
                        println!("| {} | [Source]({}) | 🔗 |", name, url);
                    }
                    Source::Other { description } => {
                        println!("| {} | {} | ❓ |", name, description);
                    }
                }
            }
        }
        _ => {
            // TODO: Implement other output formats
            return Err(anyhow::anyhow!(t!(
                "main.format_is_not_implemented_yet",
                format = format
            )));
        }
    }

    Ok(())
}
