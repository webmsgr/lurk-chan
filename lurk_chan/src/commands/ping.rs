use poise::CreateReply;

/// Ping? Pnog!
#[poise::command(slash_command)]
pub async fn ping(ctx: crate::Context<'_>) -> anyhow::Result<()> {
    ctx.send(CreateReply::default().content(format!(
        "Pnog! I'm Lurk-chan v{}! My ping to discord is {:?}",
        env!("CARGO_PKG_VERSION"),
        get_ping(&ctx).await
    )))
    .await?;
    Ok(())
}

pub async fn get_ping(ctx: &crate::Context<'_>) -> std::time::Duration {
    match ctx
        .framework()
        .shard_manager
        .runners
        .lock()
        .await
        .get(&ctx.serenity_context().shard_id)
    {
        Some(runner) => runner.latency.unwrap_or(std::time::Duration::ZERO),
        None => {
            tracing::error!("current shard is not in shard_manager.runners, this shouldn't happen");
            std::time::Duration::ZERO
        }
    }
}
