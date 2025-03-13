mod cli;
mod config;
mod errors;
mod sources;

use anyhow::Result;
use cargo_metadata::MetadataCommand;
use std::collections::HashMap;
use tracing::{debug, info, instrument};

use crate::{
    cli::{build_cli, generate_completions},
    config::{Config, OutputFormat},
    errors::AppError,
    sources::{CratesioClient, GitHubClient, Source},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let matches = build_cli().get_matches();

    // Handle completions subcommand
    if let Some(matches) = matches.subcommand_matches("completions") {
        if let Some(shell) = matches.get_one::<String>("shell") {
            generate_completions(shell)
                .map_err(|e| anyhow::anyhow!("Failed to generate completions: {}", e))?;
            return Ok(());
        }
    }

    // Initialize global config
    let config = Config::from_matches(&matches)?;
    Config::init(config)?;

    process_dependencies().await
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

    debug!("Found {} unique dependencies", deps.len());

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
                tracing::warn!("Failed to process {}: {}", name, e);
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
                description: format!("Unknown source: {}", repo_url),
            })
        }
    } else {
        Ok(Source::CratesIo {
            name: name.to_string(),
            downloads: Some(crate_info.downloads),
        })
    }
}

fn generate_output(results: &[(String, Source)], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::MarkdownTable => {
            println!("| Name | Source | Stats |");
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
            return Err(anyhow::anyhow!(
                "Output format {:?} not yet implemented",
                format
            ));
        }
    }

    Ok(())
}
