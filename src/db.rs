use crate::lc::DBConn;
use crate::report::Report;
use crate::report::ReportStatus;
use anyhow::anyhow;
use anyhow::Context as ctxtrait;
use serenity::builder::EditMessage;
use serenity::model::prelude::*;
use serenity::prelude::*;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::{query, query_as};

use crate::audit::Location;
//use std::result::Result;
use tracing::{error, instrument};
pub async fn get_report(id: i64, db: &mut DBConn) -> anyhow::Result<Option<Report>> {
    let r = query_as!(Report, "select reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status as \"report_status: ReportStatus\", server, time, claimant, audit from Reports where id = ?", id).fetch_optional(db).await?;
    Ok(r)
}

pub async fn get_action(id: i64, db: &mut DBConn) -> anyhow::Result<Option<crate::audit::Action>> {
    let r = query_as!(crate::audit::Action, "select target_id, target_username, offense, action, server as \"server: Location\", claimant, report from Actions where id = ?", id).fetch_optional(db).await?;
    Ok(r)
}
#[must_use]
pub async fn update_report_message(id: i64, db: &mut DBConn, ctx: &Context) -> anyhow::Result<()> {
    let report = match get_report(id, db).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            error!("No report for: {}", id);
            return Err(anyhow!("No report for id"));
        }
        Err(e) => {
            return Err(e);
        }
    };
    let mut m = get_report_message_from_id(id, db, ctx).await?;
    let comp = report.components(id);
    m.edit(
        &ctx,
        EditMessage::default()
            .embed(report.create_embed(id, db).await)
            .components(comp),
    )
    .await?;
    Ok(())
}

#[must_use]
pub async fn update_audit_message(id: i64, db: &mut DBConn, ctx: &Context) -> anyhow::Result<()> {
    let action = match get_action(id, db).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            error!("No action for: {}", id);
            return Err(anyhow!("No action for id"));
        }
        Err(e) => {
            error!("Error while fetching action for interaction: {}", e);
            return Err(e);
        }
    };
    let mut m = match get_audit_message_from_id(id, db, ctx).await {
        Ok(m) => m,
        Err(e) => {
            error!("Error while fetching message for interaction: {}", e);
            return Err(e);
        }
    };
    let comp = action.create_components(id, db).await;
    let embed = action.create_embed(ctx, id).await?;
    let _ = m
        .edit(&ctx, EditMessage::default().embed(embed).components(comp))
        .await?;
    Ok(())
}

#[instrument(skip(db))]
pub async fn add_report(
    r: Report,
    db: &mut DBConn,
) -> anyhow::Result<SqliteQueryResult, sqlx::Error> {
    query!("insert into Reports (reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status, server, time) values (?, ?, ?, ?, ?, ?, ?, ?)",
                r.reporter_id,
                r.reporter_name,
                r.reported_id,
                r.reported_name,
                r.report_reason,
                r.report_status,
                r.server,
                r.time
            ).execute(db).await
}

//target_id text not null,
//     target_username text not null,
//     offense text not null,
//     action text not null,
//     server text not null,
//     claimant text not null,
//     report int,
//     foreign key(report) references Reports(id)
#[instrument(skip(db))]
pub async fn add_action(
    a: crate::audit::Action,
    db: &mut DBConn,
) -> anyhow::Result<SqliteQueryResult, sqlx::Error> {
    query!("insert into Actions (target_id, target_username, offense, action, server, claimant, report) values (?, ?, ?, ?, ?, ?, ?)",
        a.target_id,
        a.target_username,
            a.offense,
            a.action,
            a.server,
            a.claimant,
            a.report
            ).execute(db).await
}

pub async fn add_report_message(id: i64, message: Message, db: &mut DBConn) -> anyhow::Result<()> {
    let mid = message.id.get().to_string();
    let cid = message.channel_id.get().to_string();
    query!(
        "insert into ReportMessages (report_id, message, channel) values (?, ?, ?)",
        id,
        mid,
        cid
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn add_action_message(id: i64, message: Message, db: &mut DBConn) -> anyhow::Result<()> {
    let mid = message.id.get().to_string();
    let cid = message.channel_id.get().to_string();
    query!(
        "insert into ActionMessages (action_id, message, channel) values (?, ?, ?)",
        id,
        mid,
        cid
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_report_message_from_id(
    rid: i64,
    db: &mut DBConn,
    ctx: &impl CacheHttp,
) -> anyhow::Result<Message> {
    let rec = query!(
        "select message, channel from ReportMessages where report_id = ?;",
        rid
    )
    .fetch_one(db)
    .await?;
    let m = MessageId::new(
        rec.message
            .parse::<u64>()
            .context("Message ID to be a u64")?,
    );
    let c = ChannelId::new(
        rec.channel
            .parse::<u64>()
            .context("Channel ID to be a u64")?,
    );
    let m = c.message(ctx, m).await?;
    Ok(m)
}

pub async fn get_audit_message_from_id(
    rid: i64,
    db: &mut DBConn,
    ctx: &impl CacheHttp,
) -> anyhow::Result<Message> {
    let rec = query!(
        "select message, channel from ActionMessages where action_id = ?;",
        rid
    )
    .fetch_one(db)
    .await?;
    let m = MessageId::new(
        rec.message
            .parse::<u64>()
            .context("Message ID to be a u64")?,
    );
    let c = ChannelId::new(
        rec.channel
            .parse::<u64>()
            .context("Channel ID to be a u64")?,
    );
    let m = c.message(ctx, m).await?;
    Ok(m)
}

pub async fn get_audit_message_from_report(
    rid: i64,
    db: &mut DBConn,
    ctx: &impl CacheHttp,
) -> anyhow::Result<Option<Message>> {
    // get the report
    let rep = get_report(rid, db).await?;
    let rep = match rep {
        Some(r) => r,
        None => return Ok(None),
    };
    if let Some(e) = rep.audit {
        let rec = query!(
            "select message, channel from ActionMessages where message = ?;",
            e
        )
        .fetch_optional(db)
        .await?;
        if let Some(r) = rec {
            let m = MessageId::new(r.message.parse::<u64>().expect("Message ID to be a u64"));
            let c = ChannelId::new(r.channel.parse::<u64>().expect("Channel ID to be a u64"));
            let m = c.message(ctx, m).await?;
            Ok(Some(m))
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    }
}
