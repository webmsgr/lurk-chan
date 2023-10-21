use std::error::Error;
use std::sync::Arc;
use serenity::all::{CommandInteraction, UserId};
use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::prelude::*;
use serenity::futures::StreamExt;
use tracing::info;

use crate::report::{Report, self, ReportStatus};
use crate::{report_from_msg, db, LurkChan};
const OWNER: UserId = UserId::new(171629704959229952);

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), Box<dyn Error + Send + Sync>> {
    if interaction.user.id != OWNER {
        interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content({
            "nuh uh!"
        }).ephemeral(true))).await?;
        return Ok(())
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content({
        "begining import, god help us all"
    }).ephemeral(true))).await?;
    info!("Begining import!");
    let mut mes = interaction.channel.as_ref().expect("fuck dms").id.messages_iter(ctx).filter_map(|i| {
        async {
            tokio::task::yield_now().await;
            match i {
                Ok(msg) => {
                    match tokio::task::spawn_blocking(move || report_from_msg(&msg)).await {
                        Ok(Ok(Some(mut r))) => {
                            r.report_status = ReportStatus::Closed;
                            Some(r)
                        },
                        _ => None
                    }
                },
                Err(_) => None
            }
        }
    }).collect::<Vec<Report>>().await;
    info!("Messages loaded. {} reports", mes.len());
    //mes.reverse();
    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    mes.reverse();
    for item in mes {
        db::add_report(item, &lc.db).await?;
    }
    info!("Import complete");
    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("import").description("rest!")
}