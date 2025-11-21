mod app;
mod cli;
mod config;
mod errors;
mod output;
mod sources;
mod travert;

use anyhow::Result;

#[macro_use]
extern crate rust_i18n;

rust_i18n::i18n!(
    "locales",
    fallback = ["zh", "en", "ja", "ko", "es", "fr", "de", "it"]
);

#[tokio::main]
async fn main() -> Result<()> {
    app::run().await
}
