use crate::audit::{Action as AuditAction, Location, DISC_AUDIT};
use crate::db::{add_action, add_action_message};
use crate::LurkChan;
use anyhow::{bail, Context as _};
use serenity::all::audit_log::Action;
use serenity::all::{
    AuditLogEntry, Change, CreateMessage, GuildId, MemberAction, MessageAction, UserId,
};
use serenity::prelude::*;
use std::sync::Arc;
use tracing::{debug, info, instrument};

#[instrument(skip(ctx, entry, guild_id))]
pub async fn on_guild_audit(
    ctx: Context,
    entry: AuditLogEntry,
    guild_id: GuildId,
) -> anyhow::Result<()> {
    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    let mut db = lc.db().await;
    match entry.action {
        Action::Member(MemberAction::Update) => {
            let things_done = entry.changes.unwrap_or_default();
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
                            AuditAction {
                                target_id: user_for.id.to_string(),
                                target_username: user_for.global_name.unwrap_or(user_for.name),
                                offense: entry
                                    .reason
                                    .as_ref()
                                    .map(|x| x.as_str())
                                    .unwrap_or_else(|| "???")
                                    .to_string(),
                                action: format!("Timeout until <t:{}:f>", new.unix_timestamp()),
                                server: Location::Discord,
                                claimant: entry.user_id.to_string(),
                                report: None,
                            }
                        } else {
                            return Ok(());
                        };

                        let r = add_action(a.clone(), &mut db)
                            .await
                            .context("failed to add action")?;
                        let a_id = r.last_insert_rowid();
                        let comp = a.create_components(a_id, &mut db).await;
                        let e = a.create_embed(&ctx, a_id).await?;
                        let m = DISC_AUDIT
                            .send_message(&ctx, CreateMessage::default().embed(e).components(comp))
                            .await?;
                        add_action_message(a_id, m, &mut db)
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
