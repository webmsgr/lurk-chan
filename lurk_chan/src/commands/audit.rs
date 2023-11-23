use std::time::Duration;

use common::{Action, Location};
use poise::{
    execute_modal, execute_modal_on_component_interaction,
    serenity_prelude::{CreateActionRow, CreateButton, CreateMessage, User},
    CreateReply,
};

use crate::{AuditModal, WhatTheFuck};
/// audit that shit!
#[poise::command(slash_command, subcommands("discord", "sl"))]
pub async fn audit(_: crate::Context<'_>) -> anyhow::Result<()> {
    // no
    Ok(())
}

/// Audit a discord user
#[poise::command(slash_command)]
pub async fn discord(
    ctx: crate::ApplicationContext<'_>,
    #[description = "The user to audit"] user: Option<User>,
    #[description = "What did they do?"] offense: Option<String>,
    #[description = "What did they get?"] action: Option<String>,
) -> anyhow::Result<()> {
    let (id, name) = match user {
        Some(i) => (Some(i.id.get()), Some(i.global_name.unwrap_or(i.name))),
        None => (None, None),
    };
    do_it(
        ctx,
        AuditModal {
            id: id.unwrap_or_default().to_string(),
            name: name.unwrap_or_default(),
            reason: offense.unwrap_or_default(),
            action: action.unwrap_or_default(),
        },
        Location::Discord,
    )
    .await
}
/// Audit a SL User
#[poise::command(slash_command)]
pub async fn sl(
    ctx: crate::ApplicationContext<'_>,
    #[description = "id"] id: Option<String>,
    #[description = "name"] name: Option<String>,
    #[description = "What did they do?"] offense: Option<String>,
    #[description = "What did they get?"] action: Option<String>,
) -> anyhow::Result<()> {
    do_it(
        ctx,
        AuditModal {
            id: id.unwrap_or_default(),
            name: name.unwrap_or_default(),
            reason: offense.unwrap_or_default(),
            action: action.unwrap_or_default(),
        },
        Location::Discord,
    )
    .await
}

async fn do_it(
    ctx: crate::ApplicationContext<'_>,
    a: AuditModal,
    loc: Location,
) -> anyhow::Result<()> {
    let res = execute_modal(ctx, Some(a), Some(Duration::from_secs(120))).await?;
    match res {
        Some(a) => {
            let a = Action {
                target_id: a.id,
                target_username: a.name,
                offense: a.reason,
                action: a.action,
                server: loc,
                claimant: ctx.author().id.get(),
                report: None,
            };
            let channel_for_msg = match &a.server {
                &Location::SL => ctx.data().config.secret_lab.audit,
                &Location::Discord => ctx.data().config.discord.audit,
            };
            let aid = ctx.data().db.add_action(a.clone()).await?;
            let m = channel_for_msg
                .send_message(
                    ctx.serenity_context,
                    CreateMessage::default()
                        .embed(
                            lurk_chan::create_action_embed(
                                &a,
                                &ctx.serenity_context,
                                aid,
                                channel_for_msg,
                            )
                            .await?,
                        )
                        .components(lurk_chan::create_action_components(aid)),
                )
                .await?;
            ctx.data()
                .db
                .add_action_message(m.channel_id.get(), m.id.get(), aid)
                .await?;
            Ok(())
        }
        None => Ok(()),
    }
}
