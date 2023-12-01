use crate::{tasks, AuditModal, LurkChan};
use anyhow::{bail, Context as _};
use common::{Action, Location, Report};
use lurk_chan::{
    create_action_components, create_action_embed, execute_modal_on_component_interaction,
    transmute_json, update_audit_message, update_report_message,
};
use poise::serenity_prelude::{
    ActivityData, ComponentInteraction, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, CreateMessage,
    EditInteractionResponse, FullEvent,
};
use poise::serenity_prelude::{
    AuditLogEntry, CacheHttp, Change, Context, MemberAction, Timestamp, UserId,
};
use poise::{serenity_prelude, FrameworkContext};
use serenity::model::guild::audit_log::Action as AuditAction;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, instrument};

#[instrument(skip_all)]
pub async fn handle(
    ctx: &Context,
    evt: &FullEvent,
    framework: FrameworkContext<'_, LurkChan, anyhow::Error>,
) -> anyhow::Result<()> {
    match evt {
        FullEvent::Ready { data_about_bot } => {
            info!(
                "And {} v{} takes the stage!",
                data_about_bot.user.name,
                env!("CARGO_PKG_VERSION")
            );
            ctx.set_activity(Some(ActivityData::watching(format!(
                "for new reports! (v{})",
                env!("CARGO_PKG_VERSION")
            ))));
            tasks::start_all_background_tasks(
                ctx.clone(),
                framework.user_data.shutdown.clone(),
                framework,
            )
            .await?;
        }
        FullEvent::Message { new_message } => {
            on_message(ctx, new_message, framework.user_data).await?;
        }
        FullEvent::InteractionCreate { interaction } => {
            if let Some(r) = interaction.as_message_component() {
                if let Err(e) = on_button(ctx, r, framework.user_data).await {
                    info!("Error handling button: {:?}", e);
                    r.create_response(
                        ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .content("Error handling button. yell at wackery please")
                                .ephemeral(true),
                        ),
                    )
                    .await?;
                    return Err(e);
                }
            }
        }
        FullEvent::GuildAuditLogEntryCreate { entry, .. } => {
            on_guild_audit(ctx, entry, framework.user_data).await?;
        }
        _ => {}
    }
    Ok(())
}

#[instrument(skip(ctx, entry, lc))]
pub async fn on_guild_audit(
    ctx: &impl CacheHttp,
    entry: &AuditLogEntry,
    lc: &LurkChan,
) -> anyhow::Result<()> {
    match entry.action {
        AuditAction::Member(MemberAction::Update) => {
            let things_done = entry.changes.clone().unwrap_or_default();
            for thing in things_done {
                match thing {
                    Change::CommunicationDisabledUntil { new, .. } => {
                        //info!("{:?}, {:?}", old, new);
                        let a = if let Some(new) = new {
                            // this is a timeout
                            //let new = new.unwrap();
                            let user_for = UserId::new(
                                entry.target_id.context("No target for comm change!")?.get(),
                            );
                            let user_for = user_for.to_user(&ctx).await?;
                            Action {
                                target_id: user_for.id.to_string(),
                                target_username: user_for.global_name.unwrap_or(user_for.name),
                                offense: entry.reason.as_deref().unwrap_or("???").to_string(),
                                action: format!(
                                    "Timeout until <t:{0}:f> (<t:{0}:R>)",
                                    new.unix_timestamp()
                                ),
                                server: Location::Discord,
                                claimant: entry.user_id.get(),
                                report: None,
                            }
                        } else {
                            return Ok(());
                        };

                        let r = lc
                            .db
                            .add_action(a.clone())
                            .await
                            .context("failed to add action")?;
                        let (emb, comp) = (
                            create_action_embed(&a, ctx, r, lc.config.discord.audit).await?,
                            create_action_components(r),
                        );

                        let m = lc
                            .config
                            .discord
                            .audit
                            .send_message(ctx, CreateMessage::default().embed(emb).components(comp))
                            .await?;

                        lc.db
                            .add_action_message(m.channel_id.get(), m.id.get(), r)
                            .await
                            .context("Failed to add action message")?;
                    }
                    _ => {}
                }
            }
        }
        _ => {
            debug!("{:?}", entry);
        }
    }
    Ok(())
}

struct WhatTheFuck<'a>(&'a serenity_prelude::Context);

impl<'a> AsRef<serenity_prelude::Context> for WhatTheFuck<'a> {
    fn as_ref(&self) -> &serenity_prelude::Context {
        self.0
    }
}
pub async fn on_button(
    ctx: &Context,
    int: &ComponentInteraction,
    lc: &LurkChan,
) -> anyhow::Result<()> {
    // piss shit anmd die
    info!("Who the fuck touched the button?");
    let (kind, id) = int
        .data
        .custom_id
        .split_once('_')
        .expect("Invalid custom id, this should never fuckign happen");
    let id: u32 = id.parse().expect("Failed to parse id, fuck!");
    //let mut m = int.message.clone();
    //
    let uid = int.user.id.get();
    match kind {
        "claim" => {
            int.defer_ephemeral(ctx).await?;
            lc.db.claim_report(id, uid).await?;
            update_report_message(ctx, id, &lc.db).await?;
        }
        "close" => {
            let report = lc
                .db
                .get_report_from_id(id)
                .await?
                .context("That report dont exist")?;
            if report.claimant.is_some_and(|i| i == uid) {
                // fuck
                let resp = execute_modal_on_component_interaction(
                    ctx,
                    int.clone(),
                    Some(AuditModal {
                        id: report.reported_id,
                        name: report.reported_name,
                        reason: report.report_reason,
                        ..Default::default()
                    }),
                    Some(Duration::from_secs(120)),
                )
                .await?;

                if resp.is_none() {
                    return Ok(());
                }
                let resp = resp.unwrap();

                // create an action from resp
                let a = Action {
                    target_id: resp.id,
                    target_username: resp.name,
                    offense: resp.reason,
                    action: resp.action,
                    server: report.location,
                    report: Some(id),
                    claimant: uid,
                };

                let channel_for_msg = match a.server {
                    Location::SL => lc.config.secret_lab.audit,
                    Location::Discord => lc.config.discord.audit,
                };
                lc.db.close_report(id).await?;
                let aid = lc.db.add_action(a.clone()).await?;
                let m = channel_for_msg
                    .send_message(
                        ctx,
                        CreateMessage::default()
                            .embed(
                                lurk_chan::create_action_embed(&a, ctx, aid, channel_for_msg)
                                    .await?,
                            )
                            .components(lurk_chan::create_action_components(aid)),
                    )
                    .await?;
                lc.db
                    .add_action_message(m.channel_id.get(), m.id.get(), aid)
                    .await?;
                update_report_message(ctx, id, &lc.db).await?;
                int.create_followup(
                    ctx,
                    CreateInteractionResponseFollowup::default()
                        .content(":+1:")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            } else {
                // that doesnt fucking belong to you, dipshit
                int.create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content("sorry buddy, that doesn't belong to you")
                            .ephemeral(true),
                    ),
                )
                .await?;
                return Ok(());
            }
        }
        "forceclose" => {
            int.defer_ephemeral(ctx).await?;
            let report = lc
                .db
                .get_report_from_id(id)
                .await?
                .context("That report dont exist")?;
            if report.claimant.is_some_and(|i| i == uid) {
                lc.db.close_report(id).await?;
                update_report_message(ctx, id, &lc.db).await?;
            } else {
                int.create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content("sorry buddy, that doesn't belong to you")
                            .ephemeral(true),
                    ),
                )
                .await?;
                return Ok(());
            }
        }
        "edit" => {
            let action = lc
                .db
                .get_action_from_id(id)
                .await?
                .context("That action do no exis")?;
            if action.claimant != uid {
                int.create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content("sorry buddy, that doesn't belong to you")
                            .ephemeral(true),
                    ),
                )
                .await?;
                return Ok(());
            }
            // lets fucking do this
            let resp = execute_modal_on_component_interaction(
                ctx,
                int.clone(),
                Some(Into::<AuditModal>::into(action.clone())),
                Some(Duration::from_secs(120)),
            )
            .await?;
            if resp.is_none() {
                return Ok(());
            }
            let resp = resp.unwrap();
            let a = Action {
                target_id: resp.id,
                target_username: resp.name,
                offense: resp.reason,
                action: resp.action,
                ..action
            };

            lc.db
                .edit_action(id, a, Timestamp::now().to_string(), uid)
                .await?;
            update_audit_message(ctx, id, &lc.db).await?;
            int.create_followup(
                ctx,
                CreateInteractionResponseFollowup::default()
                    .content(":+1:")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
        e => {
            //error!("Invalid button type: {}", e);
            bail!("Invalid button type: {}", e);
        }
    }
    int.edit_response(ctx, EditInteractionResponse::default().content(":+1:"))
        .await?;
    Ok(())
}

pub fn report_from_msg(msg: &serenity_prelude::Message) -> anyhow::Result<Option<Report>> {
    if let Some(embed) = msg.embeds.get(0) {
        if embed.title.as_deref() != Some("Player Report") {
            return Ok(None);
        }
        // this is probably a report! yay!
        let mut field_ma = HashMap::with_capacity(embed.fields.len());
        for field in &embed.fields {
            field_ma.insert(
                field.name.clone(),
                lurk_chan::do_sl_subs(&field.value).replace('`', ""),
            );
        }
        // transmute the field_ma into a Report
        //info!("{:#?}", field_ma);
        let r: Report = match transmute_json(field_ma) {
            Ok(v) => v,
            Err(err) => {
                return Err(err.into());
            }
        };
        return Ok(Some(r));
    }
    Ok(None)
}

async fn on_message(
    ctx: &impl CacheHttp,
    new_message: &serenity_prelude::Message,
    lc: &LurkChan,
) -> anyhow::Result<()> {
    if let Some(report) = report_from_msg(new_message)? {
        // holy shit this is a report!
        // add that shit to the db
        let id = lc.db.add_report(report.clone()).await?;
        // send the report message
        let (embed, comp) = lurk_chan::create_things_from_report(report, id, &lc.db).await?;
        let m = new_message
            .channel_id
            .send_message(ctx, CreateMessage::default().embed(embed).components(comp))
            .await?;
        lc.db
            .add_report_message(m.channel_id.get(), m.id.get(), id)
            .await?;
        new_message.delete(ctx).await?;
        return Ok(());
    }
    Ok(())
}
