use reqwest::Client;
use serde::Deserialize;
use url::Url;

#[derive(Debug)]
pub enum Source {
    GitHub { owner: String, repo: String },
    Cratesio,
    Link(String),
    Other,
}

impl Source {
    pub fn from_url(url: &Option<Url>) -> Option<Self> {
        url.as_ref().and_then(|u| match u.host_str()? {
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
        })
    }
}

pub struct CratesioClient(Client);

impl CratesioClient {
    pub fn new() -> Self {
        Self(Client::new())
    }

    pub async fn get_crate_info(&self, name: &str) -> anyhow::Result<Option<CrateInfo>> {
        let response = self
            .0
            .get(format!("https://crates.io/api/v1/crates/{}", name))
            .send()
            .await?;
        if response.status().is_success() {
            let info: CrateInfo = response.json().await?;
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }
}

#[derive(Deserialize)]
pub struct CrateInfo {
    pub repository: Option<Url>,
}

pub struct GitHubClient(Client);

impl GitHubClient {
    pub fn new(token: &str) -> anyhow::Result<Self> {
        let client = Client::builder()
            .user_agent("cargo-thanku")
            .bearer_auth(token)
            .build()?;
        Ok(Self(client))
    }

    pub async fn star_repository(&self, owner: &str, repo: &str) -> anyhow::Result<()> {
        let url = format!("https://api.github.com/user/starred/{}/{}", owner, repo);
        self.0.put(&url).send().await?.error_for_status()?;
        Ok(())
    }
}
