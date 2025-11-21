use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Result;
use cargo_metadata::MetadataCommand;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::sync::Semaphore;
use tracing::{debug, info, instrument};
use url::Url;

use crate::{
    config::Config,
    errors::AppError,
    output::{DependencyInfo, DependencyKind, DependencyStats, OutputFormat, OutputManager},
    sources::{CratesioClient, GitHubClient},
};

#[instrument(skip_all)]
pub async fn process_dependencies() -> Result<()> {
    let config = Config::global()?;

    let mut deps = get_dependencies(&config.get_cargo_toml_path()?)?;
    debug!("{}", t!("main.found_dependencies", count = deps.len()));

    if config.no_relative_libs {
        debug!("{}", t!("main.filtering_relative_libs"));
        deps.retain(|_, dep| dep.path.is_none());
    }

    let crates_io_client = Arc::new(CratesioClient::new());
    let github_client = match &config.github_token {
        Some(token) => Some(Arc::new(GitHubClient::new(token)?)),
        None => None,
    };

    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_requests));

    let mut tasks = FuturesUnordered::new();
    for (name, dep) in deps {
        let dependency_name = name.clone();
        let dependency_kind = dep.kind.into();
        let crates_io = Arc::clone(&crates_io_client);
        let github = github_client.as_ref().map(Arc::clone);
        let semaphore = Arc::clone(&semaphore);
        let max_retries = config.max_retries;

        tasks.push(async move {
            let _permit = semaphore.acquire_owned().await.unwrap();
            process_with_retries(
                dependency_name,
                dependency_kind,
                crates_io,
                github,
                max_retries,
            )
            .await
        });
    }

    let mut collected = Vec::new();
    while let Some(dep) = tasks.next().await {
        collected.push(dep);
    }

    generate_output(&collected, config.format)?;
    Ok(())
}

#[instrument(skip_all)]
async fn process_with_retries(
    name: String,
    dep_kind: DependencyKind,
    crates_io_client: Arc<CratesioClient>,
    github_client: Option<Arc<GitHubClient>>,
    max_retries: u32,
) -> DependencyInfo {
    let mut last_error = None;

    for retry in 0..=max_retries {
        match process_dependency(&name, dep_kind, &crates_io_client, github_client.as_deref()).await
        {
            Ok(info) => {
                if retry > 0 {
                    debug!(
                        "{}",
                        t!("main.retry_succeeded", name = name, attempt = retry + 1)
                    );
                }
                return info;
            }
            Err(err) => {
                last_error = Some(err);
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
                }
            }
        }
    }

    let error_msg = last_error.map(|err| err.to_string()).unwrap_or_else(|| {
        t!("main.max_retries_exceeded", name = &name, error = "unknown").to_string()
    });
    debug!(
        "{}",
        t!(
            "main.max_retries_exceeded",
            name = &name,
            error = &error_msg
        )
    );
    DependencyInfo::failure(&name, dep_kind, error_msg)
}

#[instrument(skip_all)]
async fn process_dependency(
    name: &str,
    dep_kind: DependencyKind,
    crates_io_client: &CratesioClient,
    github_client: Option<&GitHubClient>,
) -> Result<DependencyInfo, AppError> {
    let crate_info = crates_io_client
        .get_crate_info(name)
        .await
        .map_err(|err| AppError::Unknown(err.to_string()))?;

    let (source_type, source_url, stats) = if let Some(repo) = crate_info.repository.as_ref() {
        if let Ok(url) = Url::parse(repo) {
            if url.host_str() == Some("github.com") {
                let path_segments: Vec<&str> = url
                    .path_segments()
                    .map(|segments| segments.collect())
                    .unwrap_or_default();

                if path_segments.len() >= 2 {
                    let owner = path_segments[0];
                    let repo = path_segments[1];

                    if let Some(client) = github_client {
                        match client.get_repository_info(owner, repo).await {
                            Ok(repo_info) => {
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
        dependency_kind: dep_kind,
        description: crate_info.description,
        crate_url: Some(CratesioClient::get_crate_url(name)),
        source_type,
        source_url,
        stats,
        failed: false,
        error_message: None,
    })
}

#[instrument(skip_all)]
fn get_dependencies<P>(cargo_toml_path: P) -> Result<HashMap<String, cargo_metadata::Dependency>>
where
    P: AsRef<std::path::Path>,
{
    let metadata = MetadataCommand::new()
        .manifest_path(cargo_toml_path.as_ref())
        .no_deps()
        .exec()
        .map_err(AppError::MetadataError)?;

    let mut deps = HashMap::new();
    for pkg in &metadata.packages {
        for dep in &pkg.dependencies {
            deps.entry(dep.name.clone()).or_insert_with(|| dep.clone());
        }
    }

    debug!("{}", t!("main.found_dependencies", count = deps.len()));
    Ok(deps)
}

#[instrument(skip(deps))]
fn generate_output(deps: &[DependencyInfo], format: OutputFormat) -> Result<()> {
    let config = Config::global()?;
    let output = config.get_output_writer()?;
    let mut manager = OutputManager::new(format, output);
    manager.write(deps)
}
