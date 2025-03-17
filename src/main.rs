mod cli;
mod config;
mod errors;
mod output;
mod sources;

use anyhow::Result;
use cargo_metadata::MetadataCommand;
use output::OutputFormat;
use rust_i18n::t;
use tracing::{Level, debug, info, instrument};
use url::Url;

use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::{
    cli::{build_cli, generate_completions},
    config::Config,
    errors::AppError,
    output::{DependencyInfo, DependencyStats, OutputManager},
    sources::{CratesioClient, GitHubClient},
};

#[macro_use]
extern crate rust_i18n;

rust_i18n::i18n!(
    "locales",
    fallback = ["zh", "en", "ja", "ko", "es", "fr", "de", "it"]
);

#[tokio::main]
async fn main() -> Result<()> {
    let cli = build_cli();
    let matches = cli.get_matches_from(filter_cargo_args());

    let language = matches
        .get_one::<String>("language")
        .cloned()
        .unwrap_or("zh".to_string());
    let verbose = matches.get_flag("verbose");

    // Set locale
    rust_i18n::set_locale(&language);

    // Initialize tracing
    init_log(if verbose { Level::DEBUG } else { Level::INFO })?;

    // Initialize global config
    let config = Config::from_matches(&matches)?;
    Config::init(config)?;

    // Handle subcommand
    if let Some(matches) = matches.subcommand_matches("completions") {
        handle_completions(&matches)?;
        return Ok(());
    }

    #[cfg(debug_assertions)]
    {
        if let Some(matches) = matches.subcommand_matches("test") {
            handle_test(&matches)?;
            return Ok(());
        }
    }

    process_dependencies().await
}

#[instrument]
fn filter_cargo_args() -> impl Iterator<Item = String> {
    std::env::args().enumerate().filter_map(|(i, arg)| {
        match (i, arg.as_str()) {
            // 过滤掉 cargo 传递的子命令名（如 "thanku"、"thx"）
            (1, "thanku" | "thx" | "thxu") => None,
            _ => Some(arg),
        }
    })
}

#[instrument]
pub fn init_log(log_level: Level) -> Result<()> {
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
fn handle_completions(matches: &clap::ArgMatches) -> Result<()> {
    if let Some(shell) = matches.get_one::<String>("shell") {
        generate_completions(shell).map_err(|e| {
            anyhow::anyhow!(t!(
                "main.failed_generate_completions",
                error = e.to_string()
            ))
        })?;
        return Ok(());
    }
    Ok(())
}

#[instrument(skip_all)]
fn handle_test(matches: &clap::ArgMatches) -> Result<()> {
    tracing::info!("test: {:?}", matches);
    println!("{}", t!("app.description"));
    println!("test: {:?}", matches);
    return Ok(());
}

#[instrument(skip_all)]
fn get_dependencies<P>(cargo_toml_path: P) -> Result<HashMap<String, cargo_metadata::Dependency>>
where
    P: AsRef<std::path::Path>,
{
    // Get cargo metadata
    let metadata = MetadataCommand::new()
        .manifest_path(cargo_toml_path.as_ref())
        .no_deps() // 只获取当前包的依赖
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
    Ok(deps)
}

/// TODO: Check the concurrent requests
#[instrument(skip_all)]
async fn process_dependencies() -> Result<()> {
    let config = Config::global()?;

    // Get cargo metadata
    let deps = get_dependencies(&config.get_cargo_toml_path()?)?;
    debug!("{}", t!("main.found_dependencies", count = deps.len()));

    // Initialize clients
    let crates_io_client = Arc::new(CratesioClient::new());
    let github_client = if let Some(token) = &config.github_token {
        Some(Arc::new(GitHubClient::new(token)?))
    } else {
        None
    };

    // Create semaphore to limit concurrent requests
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_requests));
    // Create tasks
    let mut tasks: Vec<tokio::task::JoinHandle<Result<(String, DependencyInfo), AppError>>> =
        Vec::new();
    for (name, _) in deps {
        let name = name.clone();
        let crates_io_client = Arc::clone(&crates_io_client);
        let github_client = github_client.as_ref().map(Arc::clone);
        let semaphore = Arc::clone(&semaphore);
        let max_retries = config.max_retries;

        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let mut last_error = None;

            for retry in 0..=max_retries {
                match process_dependency(&name, &crates_io_client, github_client.as_deref()).await {
                    Ok(info) => {
                        if retry > 0 {
                            debug!(
                                "{}",
                                t!("main.retry_succeeded", name = name, attempt = retry + 1)
                            );
                        }
                        return Ok((name, info));
                    }
                    Err(e) => {
                        last_error = Some(e);
                        if retry < max_retries {
                            let delay = Duration::from_secs(2u64.pow(retry));
                            debug!(
                                "{}",
                                t!(
                                    "main.retry_attempt",
                                    name = name,
                                    attempt = retry + 1,
                                    max_retries = max_retries,
                                    delay = delay.as_secs()
                                )
                            );
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                    }
                }
            }

            // 创建一个表示失败的 DependencyInfo
            let error_msg = last_error.unwrap().to_string();
            debug!(
                "{}",
                t!("main.max_retries_exceeded", name = name, error = error_msg)
            );

            Ok((
                name.clone(),
                DependencyInfo {
                    name: name.clone(),
                    description: None,
                    crate_url: Some(CratesioClient::get_crate_url(&name)),
                    source_type: "Unknown".to_string(),
                    source_url: None,
                    stats: DependencyStats {
                        stars: None,
                        downloads: None,
                    },
                    failed: true,
                    error_message: Some(error_msg),
                },
            ))
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete and collect results
    let results: Vec<_> = join_all(tasks)
        .await
        .into_iter()
        .filter_map(|result| match result {
            Ok(Ok(dep_result)) => Some(dep_result),
            Ok(Err(e)) => {
                debug!("{}", t!("main.dependency_processing_failed", error = e));
                None
            }
            Err(e) => {
                debug!("{}", t!("main.task_failed", error = e));
                None
            }
        })
        .collect();

    // Generate output
    let format = config.format;
    generate_output(&results, &format)?;

    Ok(())
}

#[instrument(skip(crates_io_client, github_client))]
async fn process_dependency(
    name: &str,
    crates_io_client: &CratesioClient,
    github_client: Option<&GitHubClient>,
) -> Result<DependencyInfo> {
    // Get crate information from crates.io
    let crate_info = crates_io_client.get_crate_info(name).await?;

    // Get repository URL if available
    let (source_type, source_url, stats) = if let Some(repo) = crate_info.repository.as_ref() {
        if let Ok(url) = Url::parse(repo) {
            if url.host_str() == Some("github.com") {
                // Extract owner and repo from GitHub URL
                let path_segments: Vec<&str> = url
                    .path_segments()
                    .map(|segments| segments.collect())
                    .unwrap_or_default();

                if path_segments.len() >= 2 {
                    let owner = path_segments[0];
                    let repo = path_segments[1];

                    // Get GitHub information if client is available
                    if let Some(client) = github_client {
                        match client.get_repository_info(owner, repo).await {
                            Ok(repo_info) => {
                                // Try to star the repository
                                let _ = client.star_repository(owner, repo).await;
                                info!("💖 {} {}", name, repo_info.html_url);

                                (
                                    "GitHub".to_string(),
                                    Some(url.to_string()),
                                    DependencyStats {
                                        stars: Some(repo_info.stargazers_count),
                                        downloads: None,
                                    },
                                )
                            }
                            Err(e) => {
                                debug!("{}", t!("main.github_api_error", error = e.to_string()));
                                (
                                    "GitHub".to_string(),
                                    Some(url.to_string()),
                                    DependencyStats {
                                        stars: None,
                                        downloads: None,
                                    },
                                )
                            }
                        }
                    } else {
                        (
                            "GitHub".to_string(),
                            Some(url.to_string()),
                            DependencyStats {
                                stars: None,
                                downloads: None,
                            },
                        )
                    }
                } else {
                    (
                        "Source".to_string(),
                        Some(url.to_string()),
                        DependencyStats {
                            stars: None,
                            downloads: None,
                        },
                    )
                }
            } else {
                (
                    "Source".to_string(),
                    Some(url.to_string()),
                    DependencyStats {
                        stars: None,
                        downloads: None,
                    },
                )
            }
        } else {
            debug!("{}", t!("main.invalid_repo_url", url = repo));
            (
                "crates.io".to_string(),
                Some(format!("https://crates.io/crates/{}", name)),
                DependencyStats {
                    stars: None,
                    downloads: Some(crate_info.downloads),
                },
            )
        }
    } else {
        (
            "crates.io".to_string(),
            Some(format!("https://crates.io/crates/{}", name)),
            DependencyStats {
                stars: None,
                downloads: Some(crate_info.downloads),
            },
        )
    };

    Ok(DependencyInfo {
        name: name.to_string(),
        description: crate_info.description,
        crate_url: Some(CratesioClient::get_crate_url(name)),
        source_type,
        source_url,
        stats,
        failed: false,
        error_message: None,
    })
}

// 在 main.rs 中使用
#[instrument(skip(results))]
fn generate_output(results: &[(String, DependencyInfo)], format: &OutputFormat) -> Result<()> {
    let config = Config::global()?;

    let deps: Vec<DependencyInfo> = results
        .iter()
        .map(|(_, dep_info)| dep_info.clone())
        .collect();

    // 根据配置选择输出目标
    let output = config.get_output_writer()?;
    let mut manager = OutputManager::new(*format, output);
    manager.write(&deps)?;

    Ok(())
}
