use std::env::var;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Context;
use async_shutdown::ShutdownManager;
use futures::StreamExt;
use serenity::futures::TryStreamExt;
use once_cell::sync::Lazy;
use serenity::all::{Cache, ChannelId, CreateEmbed, EditMessage};
use serenity::builder::CreateMessage;
use serenity::model::{Color, Timestamp};
use serenity::prelude::CacheHttp;
use tokio::select;
use tracing::error;
use crate::LurkChan;


pub static STATS_CHANNEL: Lazy<ChannelId> = Lazy::new(|| var("STATS_CHANNEL").unwrap().parse().unwrap());
pub async fn stats_task(lc: Arc<LurkChan>, r_ctx: (Arc<Cache>, Arc<serenity::http::Http>), shut: ShutdownManager<&'static str>) -> anyhow::Result<()> {
    let ctx = (&r_ctx.0, r_ctx.1.http());
    while let Err(_) = ctx.1.get_current_user().await {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    tokio::time::sleep(Duration::from_millis(1500)).await;
    // we gotta get the message
    let mut s = STATS_CHANNEL.messages_iter(&ctx).boxed();
    let mut m = None;
    while let Ok(Some(e)) = s.try_next().await {
        if e.is_own(&ctx) {
            m.replace(e);
            break;
        }
    }
    let mut msg = match m {
        Some(m) => m,
        None => STATS_CHANNEL.send_message(&ctx, CreateMessage::new().content("Loading first time stats...")).await.context("Failed to send new message")?
    };
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        select! {
            _ = interval.tick() => {  },
            _ = shut.wait_shutdown_triggered() => {
                break;
            }
        }
        // collect stats.
        let mut db = lc.db().await;
        let db_data: Result<(i64, i64, i64, i64, usize, bool), sqlx::Error> = async move {
            let report_count = sqlx::query!("select count(*) as \"count: i64\" from Reports").fetch_one(&mut db).await?.count;
            let action_count = sqlx::query!("select count(*) as \"count: i64\" from Actions").fetch_one(&mut db).await?.count;
            let report_message_count = sqlx::query!("select count(*) as \"count: i64\" from ReportMessages").fetch_one(&mut db).await?.count;
            let action_message_count = sqlx::query!("select count(*) as \"count: i64\" from ActionMessages").fetch_one(&mut db).await?.count;
            let invalid_keys = sqlx::query!("PRAGMA foreign_key_check").fetch_all(&mut db).await?.len();
            let integrety_check = sqlx::query!("PRAGMA integrity_check").fetch_one(&mut db).await?.integrity_check == "ok";
            //let audit_message_count = sqlx::query!("select count(*) as \"count: i64\" from ").fetch_one(db).await.unwrap().count;
            Ok((report_count, action_count, report_message_count, action_message_count, invalid_keys, integrety_check))
        }.await;

        let db_health_embed = match db_data {
            Ok((report_count, action_count, report_message_count, action_message_count, invalid_keys, integrety_check)) => {
                let is_db_healthy = invalid_keys == 0 && integrety_check;
                CreateEmbed::new().title(format!("DB Status: {}", if is_db_healthy {
                    "Healthy"
                } else {
                    "Unhealthy"
                }))
                    .field("Report Count", report_count.to_string(), true)
                    .field("Action Count", action_count.to_string(), true)
                    .field("Report Message Count", report_message_count.to_string(), true)
                    .field("Action Message Count", action_message_count.to_string(), true)
                    .field("Foreign Key Violations", if invalid_keys == 0 { "None".to_string() } else { format!("{}", invalid_keys) }, true)
                    .field("Database integrity", if integrety_check { "OK" } else { "FUCKED" }, true)
                    .color(if is_db_healthy {
                        Color::from_rgb(0, 255, 0)
                    } else {
                        Color::from_rgb(255, 0, 0)
                    })
            }
            Err(e) => {
                error!("Error fetching DB stats: {}", e);
                CreateEmbed::new().title("DB Status: Failure").description("Oh fuck! Report this to Wackery ASAP this is fucked!")
            }
        };


        let leaderboard_entries = {
            const L_COUNT: i64 = 10;
            let mut db = lc.db().await;
            let reports = sqlx::query!("select claimant, count(*) as count from Reports where claimant is not null group by claimant order by count desc limit ?", L_COUNT)
                .fetch_all(&mut db).await;
            let actions = sqlx::query!("select claimant, count(*) as count from Actions group by claimant order by count desc limit ?;", L_COUNT)
                .fetch_all(&mut db).await;
            match (reports, actions) {
                (Ok(r), Ok(a)) => Ok((r, a)),
                e => Err({
                    if let Err(e1) = e.0 {
                        e1
                    } else if let Err(e2) = e.1 {
                        e2
                    } else {
                        unreachable!();
                    }
                })
            }
        };

        let leaderboard_embed = match leaderboard_entries {
            Ok((r, a)) => {
                CreateEmbed::new().title("Leaderboard")
                    .field("Reports", {
                        r.into_iter().map(|r| format!("* <@!{}> - {}\n", r.claimant.unwrap_or_else(|| "null".to_string()), r.count))
                            .collect::<String>()
                    }, false)
                    .field("Audits", {
                        a.into_iter().map(|r| format!("* <@!{}> - {}\n", r.claimant, r.count))
                            .collect::<String>()
                    }, false)
                    .timestamp(Timestamp::now())
                    .color(Color::BLURPLE)
            },
            Err(e) => {
                error!("Error fetching DB leaderboard: {}", e);
                CreateEmbed::new().title("Failure to fetch data for leaderboard")
                    .description("Oh fuck! Report this to Wackery ASAP this is fucked!")
            }
        };

        msg.edit(&ctx, EditMessage::new().content("Stats:").embeds(vec![db_health_embed, leaderboard_embed])).await?;
    }


    Ok(())
}