use crate::db::update_report_message;
use crate::LurkChan;
use anyhow::Context;
use async_shutdown::ShutdownManager;
use chrono::{DateTime, Utc};
use futures::{StreamExt, TryStreamExt};
use serenity::all::Cache;
use serenity::prelude::CacheHttp;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tracing::info;
use tracing::warn;

pub async fn expire_task(
    lc: Arc<LurkChan>,
    r_ctx: (Arc<Cache>, Arc<serenity::http::Http>),
    shut: ShutdownManager<&'static str>,
) -> anyhow::Result<()> {
    let ctx = (&r_ctx.0, r_ctx.1.http());
    while ctx.1.get_current_user().await.is_err() {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let mut interval = tokio::time::interval(Duration::from_secs(60 * 5));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        select! {
            _ = interval.tick() => {  },
            _ = shut.wait_shutdown_triggered() => {
                break;
            }
        }
        let mut db = lc.db().await;
        let mut q = sqlx::query!("select id, time from Reports where report_status = 'open'")
            .fetch(&mut db)
            .boxed();
        let now = chrono::Utc::now();
        let mut to_close = vec![];
        while let Some(t) = q.try_next().await? {
            let time: DateTime<Utc> = t.time.parse().context("failed to parse time!")?;

            let sins = now.signed_duration_since(time);
            if sins.num_hours() > 48 {
                to_close.push(t.id)
            }
        }
        if to_close.is_empty() {
            continue;
        }
        info!("Expiring {} reports", to_close.len());
        let mut db = lc.db().await;
        for report in to_close {
            sqlx::query!(
                "update Reports set report_status = 'expired' where id = ?",
                report
            )
            .execute(&mut db)
            .await?;
            if let Err(e) = update_report_message(report, &mut db, &ctx).await {
                warn!("Failed to update report message #{}: {}", report, e);
            }
        }
        info!("expire complete");
    }

    Ok(())
}
