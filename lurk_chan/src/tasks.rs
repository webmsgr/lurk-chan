use async_shutdown::ShutdownManager;
use poise::serenity_prelude::Context;
use tracing::{instrument, info};
#[instrument(skip(ctx, s))]
pub async fn start_all_background_tasks(ctx: Context, s: ShutdownManager<&'static str>) -> anyhow::Result<()> {
    info!("Starting background tasks");


    info!("Background tasks started");
    Ok(())
}