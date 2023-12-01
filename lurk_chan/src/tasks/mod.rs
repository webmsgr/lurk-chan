use std::{path::PathBuf, time::Duration};

use anyhow::{anyhow, Context as _};
use async_shutdown::ShutdownManager;
use chrono::{DateTime, Utc};
use poise::serenity_prelude::{CacheHttp, Context, Timestamp};
mod console;
use tokio::select;
use tracing::{error, info, instrument, warn};
mod stats;
use console::console_task;
use stats::stats_task;
macro_rules! task {
    ($task:ident, $s:expr, $framework:expr, $ctx:expr) => {
        info!("starting '{}' task", stringify!($task));
        tokio::spawn({
            let ctx = ($ctx.cache.clone(), $ctx.http.clone());
            let s = $s.clone();
            let delay_s = $s.clone();
            let lc = $framework.user_data.clone();
            delay_s
                .wrap_delay_shutdown(async move {
                    let a_ctx = (&ctx.0, ctx.1.http());
                    if let Err(e) = $task(a_ctx, lc, s).await {
                        error!("Failed to run '{}': {}", stringify!($task), e);
                    };
                })
                .expect("Not already shutting down")
        });
    };
}

#[instrument(skip(ctx, s, framework))]
pub async fn start_all_background_tasks(
    ctx: Context,
    s: ShutdownManager<&'static str>,
    framework: poise::FrameworkContext<'_, crate::LurkChan, anyhow::Error>,
) -> anyhow::Result<()> {
    info!("Starting background tasks");
    task!(optimize_db_task, s, framework, ctx);
    task!(stats_task, s, framework, ctx);
    task!(backup_task, s, framework, ctx);
    task!(expire_task, s, framework, ctx);
    task!(console_task, s, framework, ctx);
    //task!(backup_task, s, framework, ctx);
    info!("Background tasks started");
    Ok(())
}

use crate::LurkChan;

#[instrument(skip(lc, s))]
async fn optimize_db_task(
    _: impl CacheHttp,
    lc: crate::LurkChan,
    s: ShutdownManager<&'static str>,
) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        select! {
            _ = interval.tick() => {},
            _ = s.wait_shutdown_triggered() => {
                break;
            }
        }
        info!("Optimizing DB");
        if let Err(e) = lc.db.optimize().await {
            error!("Failed to optimize DB: {}!", e)
        } else {
            info!("DB optimized");
        }
    }
    Ok(())
}

#[instrument(skip(lc, s))]
async fn backup_task(
    _: impl CacheHttp,
    lc: LurkChan,
    s: ShutdownManager<&'static str>,
) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(6 * 60 * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let backup_folder = PathBuf::from(".").join("backups");
    if !backup_folder.exists() {
        if let Err(e) = tokio::fs::create_dir(&backup_folder).await {
            error!("Failed to create backups directory: {}", e);
            return Err(anyhow!(e).context("Failed to create backups directory"));
        }
    }
    loop {
        select! {
            _ = interval.tick() => {},
            _ = s.wait_shutdown_triggered() => {
                break;
            }
        }
        info!("Backing up DB");
        let now = Timestamp::now();
        let backup_file = backup_folder.join(format!("backup_{}.db", now.timestamp()));
        if let Err(e) = lc.db.backup_to(backup_file).await {
            error!("Failed to backup the DB: {}! this is probably an issue!", e);
        }
        if let Ok(mut rd) = tokio::fs::read_dir("backups").await {
            let mut items = Vec::with_capacity(24);
            while let Ok(Some(i)) = rd.next_entry().await {
                items.push(i)
            }
            items.sort_by_cached_key(|v| v.file_name());
            if items.len() > 7 * 4 {
                let oldest = items[0].file_name();
                if let Err(e) = tokio::fs::remove_file(backup_folder.join(oldest)).await {
                    error!("Failed to remove oldest backup: {}", e);
                }
                info!("Removed oldest backup")
            }
        }
        info!("DB backed up")
    }
    Ok(())
}

pub async fn expire_task(
    ctx: impl CacheHttp,
    lc: LurkChan,
    shut: ShutdownManager<&'static str>,
) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 5));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        select! {
            _ = interval.tick() => {  },
            _ = shut.wait_shutdown_triggered() => {
                break;
            }
        }
        let q = lc
            .db
            .all_reports_with_status(common::ReportStatus::Open)
            .await?;
        let now = chrono::Utc::now();
        let mut to_close = vec![];
        for (id, t) in q {
            let time: DateTime<Utc> = t.time.parse().context("failed to parse time!")?;

            let sins = now.signed_duration_since(time);
            if sins.num_hours() > 48 {
                to_close.push(id)
            }
        }
        if to_close.is_empty() {
            continue;
        }
        info!("Expiring {} reports", to_close.len());
        for report in to_close {
            if let Err(e) = lc.db.expire_report(report).await {
                warn!("Failed to close report #{}: {}", report, e);
                continue;
            }
            if let Err(e) = lurk_chan::update_report_message(&ctx, report, &lc.db).await {
                warn!("Failed to update report message #{}: {}", report, e);
            }
        }
        info!("expire complete");
    }

    Ok(())
}
