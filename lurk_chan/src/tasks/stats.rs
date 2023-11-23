use std::time::Duration;

use anyhow::Context as _;
use async_shutdown::ShutdownManager;
use common::{ReportStatus, Location};
use poise::serenity_prelude::{CacheHttp, futures::{StreamExt as _, TryStreamExt as _}, CreateMessage, CreateEmbed, Timestamp, EditMessage, Color};
use serde_json::error;
use tokio::{select, try_join};
use tracing::{info, instrument, error};

use crate::LurkChan;


#[instrument(skip(ctx,lc, shut))]
pub async fn stats_task(ctx: impl CacheHttp, lc: crate::LurkChan, shut: ShutdownManager<&'static str>) -> anyhow::Result<()> {
    info!("ayy!");
    let mut s = lc.config.discord.stats.messages_iter(ctx.http()).boxed();
    let mut m = None;
    while let Ok(Some(e)) = s.try_next().await {
        if e.is_own(ctx.cache().expect("cache")) {
            m.replace(e);
            break;
        }
    }
    let mut msg = match m {
        Some(m) => m,
        None => lc.config.discord.stats
            .send_message(
                &ctx,
                CreateMessage::new().content("Loading first time stats..."),
            )
            .await
            .context("Failed to send new message")?,
    };
    const TIME: u64 = 30;
    let mut interval = tokio::time::interval(Duration::from_secs(TIME));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut uptime = chrono::Duration::seconds(-(TIME as i64));
    loop {
        select! {
            _ = interval.tick() => {  },
            _ = shut.wait_shutdown_triggered() => {
                break;
            }
        }
        uptime = uptime + chrono::Duration::seconds(TIME as i64);

        

        let embeds = try_join!(
            changelog_embed(uptime.clone()),
            db_data_embed(&lc),
            detailed_stats_embed(&lc),
            leaderboard_embed(&lc)
        )?;

        let mut emb = vec![embeds.0, embeds.1, embeds.2, embeds.3];
        let new_last = emb.pop().expect("fuck").timestamp(Timestamp::now());
        emb.push(new_last);
        msg.edit(&ctx, EditMessage::new().content("Stats:").embeds(emb))
            .await?;
    }
    Ok(())
}


pub async fn leaderboard_embed(lc: &LurkChan) -> anyhow::Result<CreateEmbed> {
    use std::fmt::Write;
    const LEADERBOARD_LIMIT: u32 = 10;
    let (reports_leader, audit_leader) = try_join!(lc.db.leaderboard_reports(LEADERBOARD_LIMIT), lc.db.leaderboard_audit(LEADERBOARD_LIMIT))?;
    Ok(CreateEmbed::new()
    .title("Leaderboard")
    .field(
        "Reports",
        {
            reports_leader.into_iter().fold(String::new(), |mut o, r| {
                let _ = writeln!(
                    o,
                    "* <@!{}> - {}",
                    r.0,
                    r.1
                );
                o
            })
        },
        false,
    )
    .field(
        "Audits",
        {
            audit_leader.into_iter().fold(String::new(), |mut o, r| {
                let _ = writeln!(o, "* <@!{}> - {}", r.0, r.1);
                o
            })
        },
        false,
    )
    .color(Color::BLURPLE))

}

pub async fn db_data_embed(lc: &LurkChan) -> anyhow::Result<CreateEmbed> {
    let db_data = try_join!(
        lc.db.total_report_count(),
        lc.db.total_action_count(),
        lc.db.get_report_message_count(),
        lc.db.get_action_message_count(),
        lc.db.foreign_key_check(),
        async { Ok(lc.db.integrety_check().await.is_ok()) }
    );
    Ok(match db_data {
        Ok((
            report_count,
            action_count,
            report_message_count,
            action_message_count,
            invalid_keys,
            integrety_check,
        )) => {
            let is_db_healthy = invalid_keys == 0 && integrety_check;
            CreateEmbed::new()
                .title(format!(
                    "DB Status: {}",
                    if is_db_healthy {
                        "Healthy"
                    } else {
                        "Unhealthy"
                    }
                ))
                .field("Report Count", report_count.to_string(), true)
                .field("Action Count", action_count.to_string(), true)
                .field(
                    "Report Message Count",
                    report_message_count.to_string(),
                    true,
                )
                .field(
                    "Action Message Count",
                    action_message_count.to_string(),
                    true,
                )
                .field(
                    "Foreign Key Violations",
                    if invalid_keys == 0 {
                        "None".to_string()
                    } else {
                        format!("{}", invalid_keys)
                    },
                    true,
                )
                .field(
                    "Database integrity",
                    if integrety_check { "OK" } else { "FUCKED" },
                    true,
                )
                .color(if is_db_healthy {
                    Color::from_rgb(0, 255, 0)
                } else {
                    Color::from_rgb(255, 0, 0)
                })
        }
        Err(e) => {
            error!("Error fetching DB stats: {}", e);
            CreateEmbed::new()
                .title("DB Status: Failure")
                .description("Oh fuck! Report this to Wackery ASAP this is fucked!")
        }
    })
}


pub async fn detailed_stats_embed(lc: &LurkChan) -> anyhow::Result<CreateEmbed> {
    let (
        open_reports, 
        claimed_reports, 
        closed_reports,
        sl_audits,
        discord_audits,
        audits_without_report
    ) = try_join!(
        lc.db.get_report_count_by_status(ReportStatus::Open),
        lc.db.get_report_count_by_status(ReportStatus::Claimed),
        async { try_join!(lc.db.get_report_count_by_status(ReportStatus::Closed), lc.db.get_report_count_by_status(ReportStatus::Expired)).map(|i| i.0 + i.1) },
        lc.db.audit_count_from_server(Location::SL),
        lc.db.audit_count_from_server(Location::Discord),
        lc.db.audit_count_without_report(),
    )?;

    let detailed_stats_embed = CreateEmbed::new()
            .title("Detailed Stats")
            .color(Color::GOLD)
            .field("Open Reports", open_reports.to_string(), true)
            .field("Claimed Reports", claimed_reports.to_string(), true)
            .field("Closed Reports", closed_reports.to_string(), true)
            .field("SL Audits", sl_audits.to_string(), true)
            .field("Discord Audits", discord_audits.to_string(), true)
            .field(
                "Audits Without Report",
                audits_without_report.to_string(),
                true,
            );
    Ok(detailed_stats_embed)
}

pub async fn changelog_embed(uptime: chrono::Duration) -> anyhow::Result<CreateEmbed> {
    use std::fmt::Write;
    let raw_change_log = include_str!("../../../changelog.md");
        let actual_change_log = raw_change_log
            .lines()
            .map_while(|l| if l.is_empty() { None } else { Some(l) })
            .fold(String::new(), |mut o, i| {
                writeln!(o, "{}", i).unwrap();
                o
            });

        Ok(CreateEmbed::new()
            .color(Color::from((0xff, 0x6e, 0xee)))
            .title(concat!("Lurk-Chan v", env!("CARGO_PKG_VERSION")))
            .field(
                "Changelog",
                format!("```md\n{}```", actual_change_log),
                false,
            )
            .field(
                "Uptime",
                format!(
                    "{:<02}:{:<02}:{:<02}:{:<02}",
                    uptime.num_days(),
                    uptime.num_hours() % 24,
                    uptime.num_minutes() % 60,
                    uptime.num_seconds() % 60
                ),
                false,
            ))
}