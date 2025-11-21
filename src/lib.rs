#[macro_use]
extern crate rust_i18n;

rust_i18n::i18n!(
    "locales",
    fallback = ["zh", "en", "ja", "ko", "es", "fr", "de", "it"]
);

pub mod app;
pub mod cli;
pub mod config;
pub mod errors;
pub mod output;
pub mod sources;
pub mod travert;
