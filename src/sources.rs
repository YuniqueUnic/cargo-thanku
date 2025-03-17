use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::instrument;
use url::Url;

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Source {
    GitHub {
        owner: String,
        repo: String,
        stars: Option<u32>,
    },
    CratesIo {
        name: String,
        downloads: Option<u32>,
    },
    Link {
        url: String,
    },
    Other {
        description: String,
    },
}

#[allow(unused)]
impl Source {
    pub fn from_url(url: &Option<Url>) -> Option<Self> {
        url.as_ref().and_then(|u| match u.host_str()? {
            "github.com" => {
                let path = u.path().trim_matches('/');
                let mut parts = path.splitn(2, '/');
                Some(Self::GitHub {
                    owner: parts.next()?.to_string(),
                    repo: parts.next()?.trim_end_matches(".git").to_string(),
                    stars: None,
                })
            }
            "crates.io" => {
                let name = u.path().trim_matches('/').to_string();
                Some(Self::CratesIo {
                    name,
                    downloads: None,
                })
            }
            _ => Some(Self::Link { url: u.to_string() }),
        })
    }
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct CrateInfo {
    pub name: String,
    pub description: Option<String>,
    // pub repository: Option<Url>,
    pub repository: Option<String>,
    pub downloads: u32,
}

pub struct CratesioClient {
    client: Client,
}

impl CratesioClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent(concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .expect(&format!("{}", t!("sources.failed_to_create_http_client"))),
        }
    }

    pub fn get_crate_url(name: &str) -> String {
        format!("https://crates.io/crates/{}", name)
    }

    #[instrument(skip(self))]
    pub async fn get_crate_info(&self, name: &str) -> Result<CrateInfo> {
        let url = format!("https://crates.io/api/v1/crates/{}", name);
        let response = self.client.get(&url).send().await?;
        let data = response.json::<serde_json::Value>().await?;
        let crate_info = data["crate"].clone();

        Ok(serde_json::from_value(crate_info)?)
    }
}

pub struct GitHubClient {
    client: Client,
}

impl GitHubClient {
    pub fn new(token: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("token {}", token))?,
                );
                headers
            })
            .build()?;

        Ok(Self { client })
    }

    #[instrument(skip(self))]
    pub async fn star_repository(&self, owner: &str, repo: &str) -> Result<()> {
        let url = format!("https://api.github.com/user/starred/{}/{}", owner, repo);
        self.client.put(&url).send().await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get_repository_info(&self, owner: &str, repo: &str) -> Result<RepositoryInfo> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let response = self.client.get(&url).send().await?;
        Ok(response.json().await?)
    }
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct RepositoryInfo {
    pub full_name: String,
    pub description: Option<String>,
    pub stargazers_count: u32,
    pub html_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_from_github_url() {
        let url = Url::parse("https://github.com/owner/repo").unwrap();
        if let Some(Source::GitHub { owner, repo, .. }) = Source::from_url(&Some(url)) {
            assert_eq!(owner, "owner");
            assert_eq!(repo, "repo");
        } else {
            panic!("Expected GitHub source");
        }
    }

    #[test]
    fn test_source_from_cratesio_url() {
        let url = Url::parse("https://crates.io/crates/serde").unwrap();
        if let Some(Source::CratesIo { name, .. }) = Source::from_url(&Some(url)) {
            assert_eq!(name, "crates/serde");
        } else {
            panic!("Expected CratesIo source");
        }
    }

    #[test]
    fn test_source_from_link_url() {
        let url = Url::parse("https://example.com/path/to/resource").unwrap();
        if let Some(Source::Link { url: link_url, .. }) = Source::from_url(&Some(url)) {
            assert_eq!(link_url, "https://example.com/path/to/resource");
        } else {
            panic!("Expected Link source");
        }
    }

    #[test]
    fn test_source_from_other_url() {
        let url = Url::parse("https://example.com/path/to/resource").unwrap();
        if let Some(Source::Other { description, .. }) = Source::from_url(&Some(url)) {
            assert_eq!(description, "https://example.com/path/to/resource");
        }
    }

    #[tokio::test]
    async fn test_cratesio_client_serde() {
        let client = CratesioClient::new();
        let crate_info = client.get_crate_info("serde").await.unwrap();
        println!("{:?}", crate_info);
        assert_eq!(crate_info.name, "serde");
        assert!(crate_info.description.is_some());
        assert!(crate_info.repository.is_some());
        assert!(
            crate_info
                .repository
                .unwrap()
                .eq("https://github.com/serde-rs/serde")
        );
        assert!(crate_info.downloads > 0);
    }
}
