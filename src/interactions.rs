use crate::audit::{AuditModelResult, Location, DISC_AUDIT, SL_AUDIT};
use crate::db::{
    add_action, add_action_message, get_action, get_audit_message_from_report, get_report,
    update_audit_message, update_report_message,
};
use crate::prefabs::audit_log_modal;

use crate::{commands, LurkChan};
use anyhow::{anyhow, bail};
use anyhow::Context as _;
use serenity::all::EditInteractionResponse;
use serenity::builder::{
    CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
    CreateMessage,
};
use serenity::model::prelude::*;
use serenity::prelude::*;
use sqlx::query;
use std::collections::HashMap;

use std::sync::Arc;
use tracing::{error, info, instrument, warn};

pub async fn on_interaction(ctx: Context, interaction: Interaction) {
    if let Some(m) = interaction.as_message_component() {
        if let Err(e) = on_interaction_button(&ctx, m).await {
            error!("Ruh roh, an error on button! {} !", e);
            let _ = m
                .create_followup(
                    &ctx,
                    CreateInteractionResponseFollowup::default()
                        .ephemeral(true)
                        .content("Error! Contact wackery"),
                )
                .await;
            return;
        };
    } else if let Some(modl) = interaction.as_modal_submit() {
        //let _ = modl.defer_ephemeral(&ctx).await;
        if let Err(e) = on_model(&ctx, modl).await {
            error!("Ruh roh, an error on_model! {} !", e);
            let _ = modl
                .create_followup(
                    &ctx,
                    CreateInteractionResponseFollowup::default()
                        .ephemeral(true)
                        .content("Error! Contact wackery"),
                )
                .await;
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
async fn on_model(ctx: &Context, modl: &ModalInteraction) -> anyhow::Result<()> {
    info!("Judgement day");
    modl.defer_ephemeral(&ctx)
        .await
        .context("Failed to defer ephemeral")?;
    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    let mut db = lc.db().await;
    let mut c = modl.data.custom_id.chars();
    let s = c.next().ok_or_else(|| anyhow!("Custom id to be present"))?;
    let id: i64 = c
        .collect::<String>()
        .parse()
        .context("Custom id to be a number")?;
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
            (
                e.custom_id,
                e.value
                    .expect("Should be filled out because we got it from dickcord"),
            )
        })
        .collect();
    //println!("{:?}, {:?}", model_data, modl.message);
    //modl.defer_ephemeral(&ctx).await?;
    let model_data: AuditModelResult = serde_json::to_value(model_data)
        .and_then(|s| serde_json::from_value(s))
        .context("Failed to parse modal")?;
    //println!("{:?}", model_data);
    match s {
        'r' => {
            let report_id = if id > 0 { Some(id) } else { None };
            let action = model_data.to_action(report_id.clone(), u.id);
            let chan = match &action.server {
                Location::Discord => *DISC_AUDIT,
                Location::SL => *SL_AUDIT,
            };
            let r = add_action(action.clone(), &mut db)
                .await
                .context("failed to add action")?;
            let a_id = r.last_insert_rowid();
            let comp = action.create_components(a_id, &mut db).await;
            let e = action.create_embed(&ctx, a_id).await?;

            if let Some(_) = get_audit_message_from_report(id, &mut db, ctx)
                .await
                .context("Failed to get audit message from report")?
            {
                warn!("Probably a race condition, this is bad!");
                //m.edit(ctx, EditMessage::default().embed(e).components(comp)).await?;
            } else {
                let m = chan
                    .send_message(ctx, CreateMessage::default().embed(e).components(comp))
                    .await?;
                add_action_message(a_id, m.clone(), &mut db)
                    .await
                    .context("Failed to add action message")?;
                if let Some(id) = report_id {
                    //let mut report_msg = modl.message.clone().unwrap();
                    let uid = u.id.get().to_string();
                    let mid = m.id.get().to_string();
                    query!(
                        "update Reports set claimant = ?, report_status = 'closed', audit = ? where id = ?",
                        uid,
                        mid,
                        id
                    )
                    .execute(&mut db)
                    .await.context("failed to update database")?;
                    update_report_message(id, &mut db, ctx)
                        .await
                        .context("failed to update report message")?;
                }
            }
        }
        'a' => {
            let action = get_action(id, &mut db)
                .await?
                .ok_or_else(|| anyhow!("No action for id"))?;
            let mut m_action = model_data.to_action(Some(id), u.id);
            m_action.report = action.report.clone();
            m_action.server = action.server.clone();
            let old = serde_json::to_value(&action)?;
            //target_id text not null,
            //target_username text not null,
            //offense text not null,
            //action text not null,
            let _ = query!("update Actions set target_id = ?, target_username = ?, offense = ?, action = ? where id = ?", 
                m_action.target_id,
                m_action.target_username,
                m_action.offense,
                m_action.action,
                id).execute(&mut db).await?;
            let new = serde_json::to_value(&m_action)?;

            if old == new {
                // no change
                info!("no changes!");
                modl.edit_response(ctx, EditInteractionResponse::new().content("no change"))
                    .await?;
                return Ok(());
            }
            let who = u.id.to_string();
            let when = Timestamp::now().to_string();
            let changes = json_patch::diff(&old, &new);
            let old_str = serde_json::to_string(&old)?;
            let new_str = serde_json::to_string(&new)?;
            let changes_str = serde_json::to_string(&changes)?;
            query!("insert into AuditEdits (action_id, old, new, who, time, changes) values (?, ?, ?, ?, ?, ?)", id, old_str, new_str, who, when, changes_str).execute(&mut db).await?;
            update_audit_message(id, &mut db, ctx).await?;
        }
        _ => bail!("Invalid s: {}", s),
    }

    modl.edit_response(ctx, EditInteractionResponse::new().content("ok"))
        .await?;
    return Ok(());
}

#[instrument(skip(ctx, int))]
async fn on_interaction_button(ctx: &Context, int: &ComponentInteraction) -> anyhow::Result<()> {
    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    let mut db = lc.db().await;
    let (kind, id) = int
        .data
        .custom_id
        .split_once("_")
        .expect("Invalid custom id, this should never fuckign happen");
    let id: i64 = id.parse().expect("Failed to parse id, fuck!");
    //let mut m = int.message.clone();
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
            .execute(&mut db)
            .await?;
        }
        "close" => {
            info!("close {}", id);
            let report = match get_report(id, &mut db).await.ok() {
                Some(Some(v)) => Some(v),
                _ => None,
            };
            if report
                .as_ref()
                .is_some_and(|e| e.claimant != Some(int.user.id.to_string()))
            {
                // nah chief
                int.create_followup(
                    ctx,
                    CreateInteractionResponseFollowup::default()
                        .content("sorry oomfie, that doesn't belong to you")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }
            int.create_response(
                &ctx,
                audit_log_modal(Some(id), 'r', report.map(|r| r.into())),
            )
            .await?;
            return Ok(());
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
        "edit" => {
            info!("edit {}", id);
            let action = match get_action(id, &mut db).await.ok() {
                Some(Some(v)) => Some(v),
                _ => None,
            };
            if let Some(a) = action {
                if a.claimant != int.user.id.get().to_string() {
                    int.create_response(
                        ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .content("You can't edit this!")
                                .ephemeral(true),
                        ),
                    )
                    .await?;
                    return Ok(());
                }
                int.create_response(&ctx, audit_log_modal(Some(id), 'a', Some(a.into())))
                    .await?;
                return Ok(());
            }
        }
        "forceclose" => {
            int.defer_ephemeral(&ctx).await?;
            let report = match get_report(id, &mut db).await.ok() {
                Some(Some(v)) => Some(v),
                _ => None,
            };
            if report
                .as_ref()
                .is_some_and(|e| e.claimant != Some(int.user.id.to_string()))
            {
                // nah chief
                int.create_followup(
                    ctx,
                    CreateInteractionResponseFollowup::default()
                        .content("sorry oomfie, that doesn't belong to you")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }
            query!(
                "update Reports set claimant = ?, report_status = 'closed' where id = ?",
                uid,
                id
            )
            .execute(&mut db)
            .await?;
        }
        _ => anyhow::bail!("Invalid interaction: {}", kind),
    }
    update_report_message(id, &mut db, &ctx).await?;
    int.create_followup(
        ctx,
        CreateInteractionResponseFollowup::default()
            .content("ok")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
