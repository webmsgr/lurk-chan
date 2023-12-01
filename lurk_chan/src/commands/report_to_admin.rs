use anyhow::Context;
use common::Report;
use poise::{
    serenity_prelude::{CreateMessage, Timestamp},
    CreateReply, Modal,
};
use std::time::Duration;
/// fuck
#[poise::command(context_menu_command = "Report Message to Staff")]
pub async fn report_to_admins(
    ctx: crate::ApplicationContext<'_>,
    message: poise::serenity_prelude::Message,
) -> anyhow::Result<()> {
    let resp: Option<ReportModal> =
        lurk_chan::execute_modal(ctx, None, Some(Duration::from_secs(300))).await?;

    if let Some(r) = resp {
        let report = Report {
            reporter_id: ctx.author().id.get().to_string(),
            reporter_name: ctx
                .author()
                .global_name
                .as_ref()
                .unwrap_or(&ctx.author().name)
                .to_string(),
            reported_id: message.author.id.get().to_string(),
            reported_name: message
                .author
                .global_name
                .as_ref()
                .unwrap_or(&message.author.name)
                .to_string(),
            report_reason: format!("{} ({})", r.why, message.link()),
            report_status: common::ReportStatus::Open,
            server: message
                .channel(ctx.serenity_context())
                .await
                .context("in channel???")?
                .guild()
                .context("not in a guild")?
                .guild_id
                .name(ctx.serenity_context())
                .context("no guiuld???")?
                .to_string(),
            time: Timestamp::now().to_string(),
            claimant: None,
            location: common::Location::Discord,
        };
        let lc = ctx.data();
        let channel_id = lc.config.discord.reports;
        let id = lc.db.add_report(report.clone()).await?;
        // send the report message
        let (embed, comp) = lurk_chan::create_things_from_report(report, id, &lc.db).await?;
        let m = channel_id
            .send_message(
                ctx.serenity_context(),
                CreateMessage::default().embed(embed).components(comp),
            )
            .await?;
        lc.db
            .add_report_message(m.channel_id.get(), m.id.get(), id)
            .await?;
        ctx.send(
            CreateReply::default()
                .content("Report sent! We may or may not get back to you in 3-5 business years.")
                .ephemeral(true),
        )
        .await?;
    }
    Ok(())
}

#[derive(Debug, Modal)]
#[name = "Report Message"]
struct ReportModal {
    #[name = "Report Reason"]
    #[placeholder = "Why?"]
    #[paragraph]
    #[min_length = 25]
    why: String,
}
