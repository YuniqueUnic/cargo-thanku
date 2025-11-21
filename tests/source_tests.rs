use cargo_thanku::sources::{CratesioClient, Source};
use url::Url;

#[test]
fn source_from_github_url() {
    let url = Url::parse("https://github.com/owner/repo").unwrap();
    match Source::from_url(&Some(url)) {
        Some(Source::GitHub { owner, repo, .. }) => {
            assert_eq!(owner, "owner");
            assert_eq!(repo, "repo");
        }
        _ => panic!("expected GitHub source"),
    }
}

#[test]
fn source_from_cratesio_url() {
    let url = Url::parse("https://crates.io/crates/serde").unwrap();
    match Source::from_url(&Some(url)) {
        Some(Source::CratesIo { name, .. }) => assert_eq!(name, "crates/serde"),
        _ => panic!("expected crates source"),
    }
}

#[test]
fn source_from_link_url() {
    let url = Url::parse("https://example.com/path/to/resource").unwrap();
    match Source::from_url(&Some(url)) {
        Some(Source::Link { url }) => assert_eq!(url, "https://example.com/path/to/resource"),
        _ => panic!("expected generic link"),
    }
}

#[tokio::test]
#[ignore = "requires crates.io network access"]
async fn cratesio_client_fetches_crate() {
    let client = CratesioClient::new();
    let info = client.get_crate_info("serde").await.unwrap();
    assert_eq!(info.name, "serde");
    assert!(info.downloads > 0);
}
