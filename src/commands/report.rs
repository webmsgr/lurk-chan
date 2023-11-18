use crate::report::ReportStatus;
use crate::LurkChan;
use crate::Report;
use serenity::all::{
    CommandInteraction, CommandOptionType, CreateCommandOption, EditInteractionResponse,
};
use serenity::builder::CreateCommand;
use serenity::prelude::*;
use sqlx::query_as;
use sqlx::Acquire;
use std::sync::Arc;
pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<()> {
    interaction.defer_ephemeral(ctx).await?;
    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    let mut db = lc.db().await;
    // grab the first argument
    let id = interaction
        .data
        .options
        .get(0)
        .expect("Option to be present")
        .value
        .as_i64()
        .expect("Value to be a string");
    let report = query_as!(Report, "select reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status as \"report_status: ReportStatus\", server, time, claimant, audit from Reports where id = ?", id).fetch_optional(db.acquire().await?).await?;
    if let Some(r) = report {
        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new().embed(r.create_embed(id, &mut db).await),
            )
            .await?;
    } else {
        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new().content("Unknown report!"),
            )
            .await?;
    }
    Ok(())
}

pub fn register() -> (CreateCommand, &'static str) {
    (
        CreateCommand::new("report")
            .description("Get a report!")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "report_id",
                    "Id of the report",
                )
                .required(true),
            ),
        "report",
    )
}
