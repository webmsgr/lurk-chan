use serenity::all::{CommandInteraction, UserId};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::futures::StreamExt;
use serenity::prelude::*;
use std::sync::Arc;
use tracing::info;

use crate::report::{Report, ReportStatus};
use crate::{db, report_from_msg, LurkChan};
const OWNER: UserId = UserId::new(171629704959229952);

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<()> {
    if interaction.user.id != OWNER {
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("nuh uh!")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }
    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("begining import, god help us all")
                    .ephemeral(true),
            ),
        )
        .await?;
    info!("Begining import!");
    let mut mes = interaction
        .channel
        .as_ref()
        .expect("fuck dms")
        .id
        .messages_iter(ctx)
        .filter_map(|i| async {
            tokio::task::yield_now().await;
            match i {
                Ok(msg) => match tokio::task::spawn_blocking(move || report_from_msg(&msg)).await {
                    Ok(Ok(Some(mut r))) => {
                        r.report_status = ReportStatus::Closed;
                        Some(r)
                    }
                    _ => None,
                },
                Err(_) => None,
            }
        })
        .collect::<Vec<Report>>()
        .await;
    info!("Messages loaded. {} reports", mes.len());
    //mes.reverse();
    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    mes.reverse();
    let mut db = lc.db().await;
    for item in mes {
        db::add_report(item, &mut db).await?;
    }
    info!("Import complete");
    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("import").description("rest!")
}
