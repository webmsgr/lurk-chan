use poise::{
    serenity_prelude::{CreateActionRow, CreateButton},
    CreateReply,
};
/// Ping? Pnog!
#[poise::command(slash_command)]
pub async fn ping(ctx: crate::Context<'_>) -> anyhow::Result<()> {
    ctx.send(CreateReply::default().content(format!(
        "Pnog! I'm Lurk-chan v{}!",
        env!("CARGO_PKG_VERSION")
    )))
    .await?;
    Ok(())
}
