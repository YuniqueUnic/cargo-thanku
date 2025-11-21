pub mod dependency;
pub mod format;
mod manager;
pub mod markdown;

pub use dependency::{DependencyInfo, DependencyKind, DependencyStats};
pub use format::{Formatter, OutputFormat};
pub use manager::OutputManager;
