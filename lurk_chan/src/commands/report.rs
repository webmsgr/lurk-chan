use poise::CreateReply;
/// Get a report by its id
#[poise::command(slash_command)]
pub async fn report(
    ctx: crate::Context<'_>,
    #[description = "Report to get"] report_id: u32,
) -> anyhow::Result<()> {
    let report = ctx.data().db.get_report_from_id(report_id).await?;
    match report {
        Some(r) => {
            let embed = lurk_chan::create_report_embed(&r, report_id, &ctx.data().db).await?;
            ctx.send(
                CreateReply::default()
                    .content(format!("Report #{}:", report_id))
                    .embed(embed)
                    .ephemeral(true),
            )
            .await?;
        }
        None => {
            ctx.send(
                CreateReply::default()
                    .content(format!("Report #{} not found!", report_id))
                    .ephemeral(true),
            )
            .await?;
        }
    }
    Ok(())
}
