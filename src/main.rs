use anyhow::Result;
use cargo_thanku::app;

#[tokio::main]
async fn main() -> Result<()> {
    app::run().await
}
