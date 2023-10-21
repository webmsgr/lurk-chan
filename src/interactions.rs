use crate::audit::{AuditModelResult, DISC_AUDIT, Location, SL_AUDIT};
use crate::db::{add_action, get_report, update_report_message};
use crate::prefabs::audit_log_modal;
use crate::{commands, LurkChan};
use serenity::model::prelude::*;
use serenity::prelude::*;
use sqlx::query;
use std::collections::HashMap;
use std::sync::Arc;
use serenity::builder::{CreateEmbed, CreateInteractionResponseFollowup, CreateInteractionResponseMessage, CreateMessage};
use tracing::{error, info, instrument};
use std::result::Result;
use serenity::all::{CreateInteractionResponse, EditInteractionResponse};

pub async fn on_interaction(ctx: Context, interaction: Interaction) {
    if let Some(m) = interaction.as_message_component() {
        if let Err(e) = on_interaction_button(&ctx, m).await {
            error!("Ruh roh, an error on button! {} !", e);
            let _ = m.create_followup(&ctx, CreateInteractionResponseFollowup::default().ephemeral(true).content("Error! Contact wackery")).await;
            return;
        };
    } else if let Some(modl) = interaction.as_modal_submit() {
        info!("Judgement day");
        //let _ = modl.defer_ephemeral(&ctx).await;
        if let Err(e) = on_model(&ctx, modl).await {
            error!("Ruh roh, an error on_model! {} !", e);
            let _ = modl.create_followup(&ctx, CreateInteractionResponseFollowup::default().ephemeral(true).content("Error! Contact wackery")).await;
            return;
        };
    } else if let Some(command) = interaction.as_command() {
        commands::run_command(&ctx, command).await;
    } else {
        info!("Unknown interaction: {:?}", interaction);
        return;
    }

}

#[instrument(skip(ctx, modl))]
async fn on_model(ctx: &Context, modl: &ModalInteraction) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    modl.defer_ephemeral(&ctx).await?;
    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    let db = &lc.db;
    let id: i64 = modl.data.custom_id.parse()?;
    let u = &modl.user;
    // send relevent message in audit log
    let model_data: HashMap<String, String> = modl
        .data
        .components
        .iter()
        .flat_map(|i| i.components.clone())
        .map(|e| {
            let e = match e {
                ActionRowComponent::InputText(t) => t,
                _ => unreachable!(),
            };
            (e.custom_id, e.value.expect("Should be filled out because we got it from dickcord"))
        })
        .collect();
    //println!("{:?}, {:?}", model_data, modl.message);
    let _ = modl.defer_ephemeral(&ctx).await;
    let model_data: AuditModelResult = serde_json::to_value(model_data).and_then(|s| serde_json::from_value(s))?;
    //println!("{:?}", model_data);
    let report_id = if id > 0 { Some(id) } else { None };
    let action = model_data.to_action(report_id.clone(), u.id);
    let chan = match &action.server {
        Location::Discord => *DISC_AUDIT,
        Location::SL => *SL_AUDIT
    };
    add_action(action.clone(), db).await?;
    let e = action.create_embed(&ctx).await?;
    let m = chan.send_message(ctx, CreateMessage::default().embed(e)).await?;
    if let Some(id) = report_id {
        let mut report_msg = modl.message.clone().unwrap();
        let uid = u.id.get().to_string();
        let mid = m.id.get().to_string();
        query!(
                "update Reports set claimant = ?, report_status = 'closed', audit = ? where id = ?",
                uid,
                mid,
                id
            )
            .execute(db)
            .await?;
        update_report_message(id, db, &mut report_msg, ctx).await;
    }
    modl.edit_response(ctx, EditInteractionResponse::new().content("ok")).await?;
    return Ok(())
}

#[instrument(skip(ctx, int))]
async fn on_interaction_button(ctx: &Context, int: &ComponentInteraction) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    let db = &lc.db;
    let (kind, id) = int
        .data
        .custom_id
        .split_once("_")
        .expect("Invalid custom id, this should never fuckign happen");
    let id: i64 = id.parse().expect("Failed to parse id, fuck!");
    let mut m = int.message.clone();
    //
    let uid = int.user.id.to_string();
    match kind {
        "claim" => {
            int.defer_ephemeral(ctx).await?;
            info!("claiming {}", id);
            query!(
                "update Reports set claimant = ?, report_status = 'claimed' where id = ?",
                uid,
                id
            )
            .execute(db)
            .await?;
        }
        "close" => {
            info!("close {}", id);
            let report = match get_report(id, db).await.ok() {
                Some(Some(v)) => Some(v),
                _ => None,
            };
            int.create_response(&ctx, audit_log_modal(Some(id), report, Location::SL)).await?;
            return Ok(())
            /*if let Err(e) = query!(
                "update Reports set claimant = ?, report_status = 'closed' where id = ?",
                uid,
                id
            )
                .execute(db)
                .await
            {
                error!("error updating db: {:?}", e);
                return;
            }*/
        }
        "forceclose" => {
            int.defer_ephemeral(&ctx).await?;
            query!(
                "update Reports set claimant = ?, report_status = 'closed' where id = ?",
                uid,
                id
            )
            .execute(db)
            .await?;
        }
        _ => unreachable!(),
    }
    update_report_message(id, db, &mut m, &ctx).await;
    int.create_followup(ctx, CreateInteractionResponseFollowup::default().content("ok").ephemeral(true)).await?;
    Ok(())
}
