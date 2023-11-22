use anyhow::Context;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init(); 
    info!("Hello, world!");
    let _db = database::Database::new().await.context("Failed to create db")?;

    Ok(())
}
