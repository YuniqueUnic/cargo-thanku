use anyhow::Context;
use clap::Command;
use tracing_subscriber::FmtSubscriber;

mod cli;
mod config;
mod errors;
mod sources;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化 tracing 日志系统
    FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // 解析 CLI 参数并初始化全局配置
    let config = config::Config::global()?;
    tracing::info!("Configuration loaded: {:?}", config);

    // 处理依赖并生成感谢结构
    process_dependencies(&config).await?;

    Ok(())
}

async fn process_dependencies(config: &config::Config) -> anyhow::Result<()> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(&config.input)
        .exec()
        .context("Failed to get cargo metadata")?;

    let deps: Vec<_> = metadata
        .packages
        .iter()
        .flat_map(|pkg| pkg.dependencies.iter().map(|d| &d.name))
        .collect();

    tracing::debug!("Found {} dependencies", deps.len());

    let crates_io_client = sources::CratesioClient::new();
    let github_client = sources::GitHubClient::new(&config.token)?;

    let mut output_data = Vec::new();
    for dep in deps {
        if let Some(source) = crates_io_client
            .get_crate_info(dep)
            .await?
            .and_then(|krate| sources::Source::from_url(&krate.repository))
        {
            match source {
                sources::Source::GitHub { owner, repo } => {
                    github_client.star_repository(&owner, &repo).await?;
                    output_data.push(format!("{} - github.com/{}/{}", dep, owner, repo));
                }
                sources::Source::Cratesio => {
                    output_data.push(format!("{} - crates.io", dep));
                }
                sources::Source::Link(url) => {
                    output_data.push(format!("{} - {}", dep, url));
                }
                sources::Source::Other => {
                    output_data.push(format!("{} - unknown source", dep));
                }
            }
        }
    }

    // 根据配置生成输出文件
    match config.output_format {
        config::OutputFormat::Toml => {
            let output = toml::to_string_pretty(&output_data)?;
            std::fs::write(&config.output_file, output)?;
        }
        config::OutputFormat::Json => {
            let output = serde_json::to_string_pretty(&output_data)?;
            std::fs::write(&config.output_file, output)?;
        }
        config::OutputFormat::MarkdownTable => {
            let table = output_data
                .iter()
                .fold(String::new(), |acc, line| acc + &format!("| {} |\n", line));
            std::fs::write(&config.output_file, table)?;
        }
        _ => {}
    }

    Ok(())
}
