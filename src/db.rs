use crate::report::Report;
use crate::report::ReportStatus;
use serenity::builder::{CreateEmbed, CreateMessage, EditMessage};
use serenity::model::prelude::*;
use serenity::prelude::*;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::{query, query_as, Error, SqlitePool};
use std::result::Result;
use serenity::futures::stream::BoxStream;
use tracing::{error, instrument};
pub async fn get_report(
    id: i64,
    db: &SqlitePool,
) -> Result<Option<Report>, Box<dyn std::error::Error + Send + Sync>> {
    let r = query_as!(Report, "select reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status as \"report_status: ReportStatus\", server, time, claimant, audit from Reports where id = ?", id).fetch_optional(db).await?;
    Ok(r)
}


pub async fn update_report_message(id: i64, db: &SqlitePool, m: &mut Message, ctx: &Context) {
    let report = match get_report(id, db).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            let _ = m.delete(&ctx).await;
            return;
        }
        Err(e) => {
            error!("Error while fetching report for interaction: {}", e);
            return;
        }
    };
    let comp = report.components(id);
    let _ = m
        .edit(&ctx, EditMessage::default().embed(report.create_embed()).components(comp))
        .await;
}
#[instrument(skip(db))]
pub async fn add_report(r: Report, db: &SqlitePool) -> Result<SqliteQueryResult, Error> {
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
pub async fn add_action(a: crate::audit::Action, db: &SqlitePool) -> Result<SqliteQueryResult, Error> {
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
