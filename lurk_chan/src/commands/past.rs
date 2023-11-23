use poise::{
    serenity_prelude::{CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter},
    CreateReply,
};
/// Look into the past, like some sort of time traveler
#[poise::command(slash_command)]
pub async fn past(ctx: crate::ApplicationContext<'_>, #[description = "who?"] who: String) -> anyhow::Result<()> {
    let info = ctx.data().db.collect_user_info(&who).await?;


    let reported_embed = CreateEmbed::default()
        .title(format!("Reports against {}", who))
        .description(info.preview_reported.into_iter().fold(String::new(), |mut o, (id, i)| {
            o.push_str(&format!("* Reported by {} ({}) for '{}' ({})\n", i.reporter_name, i.reporter_id, i.report_reason, id));
            o
        }))
        .footer(CreateEmbedFooter::new(format!("{} reports", info.times_reported)));

    let reporter_embed = CreateEmbed::default()
        .title(format!("Reports by {}", who))
        .description(info.preview_reported_others.into_iter().fold(String::new(), |mut o, (id, i)| {
            o.push_str(&format!("* Reported {} ({}) for '{}' ({})\n", i.reported_name, i.reported_id, i.report_reason, id));
            o
        }))
        .footer(CreateEmbedFooter::new(format!("{} reports", info.times_reported)));

    let action_embed = CreateEmbed::default()
        .title(format!("Actions against {}", who))
        .description(info.preview_actioned.into_iter().fold(String::new(), |mut o, (id, i)| {
            o.push_str(&format!("* {} for '{}' ({})\n", i.action, i.offense, id));
            o
        }))
        .footer(CreateEmbedFooter::new(format!("{} actions", info.times_actioned)));

    ctx.send(CreateReply::default()
        .content(format!("Past reports and actions for {}", who))
        .embed(reported_embed)
        .embed(reporter_embed)
        .embed(action_embed)
        .ephemeral(true)
    ).await?;
    Ok(())
}
