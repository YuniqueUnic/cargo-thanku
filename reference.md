```rust
// src/main.rs
mod cli;
mod errors;
mod sources;

use anyhow::{Context, Result};
use clap::Parser;
use sources::DependencySource;
use tracing::{debug, info, instrument};

use crate::{
    cli::Cli,
    errors::AppError,
    sources::{CratesioClient, GitHubClient},
};


#[macro_use]
extern crate rust_i18n;

rust_i18n::i18n!(
    "locales",
    fallback = ["en", "ja", "ko", "es", "fr", "de", "it"]
);


#[tokio::main]
async fn main() -> Result<()> {
    // 初始化 tracing 日志系统

   init_log(Level::Info);

    let cli = Cli::parse();
    process_dependencies(&cli).await
}

#[instrument(level = "INFO")]
pub fn init_log(log_level: Level) -> Result<()> {
    // 初始化日志
    let mut log_fmt = fmt()
        .with_env_filter(
            EnvFilter::builder()
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
async fn process_dependencies(cli: &Cli) -> Result<()> {
    // 获取元数据
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .context("Failed to get cargo metadata")?;

    // 收集所有依赖
    let deps: Vec<_> = metadata
        .packages
        .iter()
        .flat_map(|pkg| pkg.dependencies.iter().map(|d| &d.name))
        .collect();

    debug!("Found {} dependencies", deps.len());

    // 初始化客户端
    let crates_io_client = CratesioClient::new();
    let github_client = GitHubClient::new(&cli.token)?;

    // 处理每个依赖
    for dep in deps {
        match crates_io_client.get_crate_info(dep).await {
            Ok(krate) => {
                if let Some(source) = DependencySource::from_url(&krate.repository) {
                    handle_source(&github_client, dep, source).await?;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get info for {}: {}", dep, e);
            }
        }
    }

    Ok(())
}

#[instrument(skip(github_client))]
async fn handle_source(
    github_client: &GitHubClient,
    dep_name: &str,
    source: DependencySource,
) -> Result<()> {
    match source {
        DependencySource::GitHub { owner, repo } => {
            github_client
                .star_repository(&owner, &repo)
                .await
                .map_err(|e| AppError::GitHubOperation(e.into()))?;

            info!(
                "💖 {} {}",
                dep_name,
                yansi::Paint::rgb(128, 128, 128, format!("github.com/{}/{}", owner, repo)),
            );
        }
        DependencySource::Cratesio => {
            debug!("Crates.io dependency: {}", dep_name);
        }
        DependencySource::Link(url) => {
            debug!("Link source dependency: {} ({})", dep_name, url);
        }
    }

    Ok(())
}


// src/errors.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("GitHub API error: {0}")]
    GitHubOperation(#[source] anyhow::Error),
    
    #[error("Crates.io API error: {0}")]
    CratesioError(#[from] reqwest::Error),
    
    #[error("Metadata error: {0}")]
    MetadataError(#[from] cargo_metadata::Error),
}

// src/sources.rs
use serde::Deserialize;
use url::Url;

#[derive(Debug)]
pub enum DependencySource {
    GitHub { owner: String, repo: String },
    Cratesio,
    Link(String),
}

impl DependencySource {
    pub fn from_url(url: &Option<Url>) -> Option<Self> {
        url.as_ref().and_then(|u| {
            match u.host_str()? {
                "github.com" => {
                    let path = u.path().trim_matches('/');
                    let mut parts = path.splitn(2, '/');
                    Some(Self::GitHub {
                        owner: parts.next()?.to_string(),
                        repo: parts.next()?.trim_end_matches(".git").to_string(),
                    })
                }
                "crates.io" => Some(Self::Cratesio),
                _ => Some(Self::Link(u.to_string())),
            }
        })
    }
}

// src/cli.rs
use clap::{Arg, ArgAction, Command};

pub fn build_cli() -> Command {
    Command::new("cargo-thanku")
        .about("Give thanks to your Rust dependencies")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("token")
                .long("token")
                .env("GITHUB_TOKEN")
                .required(true)
                .action(ArgAction::Set)
                .help("GitHub authentication token"),
        )
        .subcommand(
            Command::new("completions")
                .about("Generate shell completions")
                .arg(Arg::new("shell").required(true)),
        )
}

// 生成补全脚本示例
fn generate_completions(shell: &str) {
    let mut cmd = cli::build_cli();
    clap_complete::generate(
        shell.parse().expect("Invalid shell type"),
        &mut cmd,
        "cargo-thanks",
        &mut std::io::stdout(),
    );
}

```

我的目的：

构建一个可以获取项目 dependencies 的 cli 程序，然后从而可以构建出一个 thanks/acknowledges 的 structure text (markdown table/list/ json/toml/..etc), 具体输出类型由用户输入的 CLI 参数决定。

```bash
# 使用示例 (检索../my-rust-proj的dependencies, 并且输出 toml 格式的 structure 内容到 ./thanks.toml)
# i, input: 输入(优先级100)     [default: cargo.toml]
# o, output: 输出文件(优先级90) [default: thanks.md]
# n, name: 输出文件名(优先级80) [default: thanks]
# t, type: 输出 structure 内容格式(优先级99) [default: markdown-table] [maybe: markdown-table, markdown-list, json, toml, yaml]
# l, link: dependencies 的 link 来源, [default: github] [fallback: crates-io, link-empty, other]
cargo thanku -- -i ../my-rust-proj -o ./thanks.toml 
cargo thanku -- -i ../my-rust-proj/cargo.toml -n ./thanks -t toml 
cd ../my-rust-proj/ && cargo thanku -- -t toml 
cat ../my-rust-proj/cargo.toml | cargo thanku -- -t toml -l github 
```

0. 改写成现代化的 rust, rust 版本 > 1.8.0 的写法
1. 帮我结构化这个程序，并且将 uri source 设置为 
    ```rust
    enum Source {
            Github(..),
            Cratesio(...),
            Link(...)
            Other,
            ,....
    }
    ```
2. 并且用上 clap 的 builder 模式，不用 derive 模式构建 cli, 以及 clap 的 completions 
3. 将 hyper 改成 rustls 的 reqwest, 然后不用 env_logger, 改成 tracing, tracing_subscriber, 美观 log 输出。
4. 将 error-chain 改成 anyhow, thiserror
5. 改用 serde, serde_json 等更加现代化的 crate 来改进重写这部分代码
6. 将 cli 的 args 抽离到一个 Config 里，全局单例，方便后续代码直接 `let config = Config::global()?;` 的方式调用。
7. 尽量重构代码，使其逻辑清晰易于拓展。