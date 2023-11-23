use std::{time::Duration, path::PathBuf};

use anyhow::anyhow;
use async_shutdown::ShutdownManager;
use poise::serenity_prelude::{Context, CacheHttp, Timestamp};
use serde_json::error;
use tokio::select;
use tracing::{info, instrument, error};
mod stats;
use stats::stats_task;
macro_rules! task {
    ($task:ident, $s:expr, $framework:expr, $ctx:expr) => {
        info!("starting '{}' task", stringify!($task));
          tokio::spawn({
            let ctx = ($ctx.cache.clone(), $ctx.http.clone());
            let s = $s.clone();
            let delay_s = $s.clone();
            let lc = $framework.user_data.clone();
            delay_s.wrap_delay_shutdown(async move {
                let a_ctx = (&ctx.0, ctx.1.http());
                if let Err(e) = $task(a_ctx, lc, s).await {
                    error!("Failed to run '{}': {}", stringify!($task), e);
                };
            }).expect("Not already shutting down")}
        );
    }
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
    //task!(backup_task, s, framework, ctx);
    info!("Background tasks started");
    Ok(())
}

use crate::LurkChan;

#[instrument(skip(lc, s))]
async fn optimize_db_task(_: impl CacheHttp, lc: crate::LurkChan, s: ShutdownManager<&'static str>) -> anyhow::Result<()> {
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
async fn backup_task(_: impl CacheHttp, lc: LurkChan, s: ShutdownManager<&'static str>) -> anyhow::Result<()> {
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
        if let Err(e) = lc.db.backup_to(backup_file)
        .await
        {
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