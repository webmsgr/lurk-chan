use std::sync::Arc;
use anyhow::{bail, Context as _};
use serenity::all::{CommandInteraction, CommandOptionType, CreateCommandOption, CreateEmbed, Timestamp};
use serenity::builder::{CreateCommand, CreateEmbedFooter, EditInteractionResponse};
use serenity::model::Color;
use crate::report::ReportStatus;
use serenity::prelude::*;
use sqlx::query_file;
use crate::audit::Action;
use crate::LurkChan;
use crate::report::Report;
use crate::audit::Location;
#[derive(Debug)]
enum WhereTheFuck {
    Actions,
    Reports
}

enum Resultz {
    Actions((Vec<(i64, Action)>, i64)),
    Reports((Vec<(i64, Report)>, i64))
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<()> {
    interaction.defer_ephemeral(&ctx).await?;
    let whr = match interaction.data.options.get(0).context("No option found for where")?
        .value
        .as_str().context("not a fucking string")? {
        "actions" => WhereTheFuck::Actions,
        "reports" => WhereTheFuck::Reports,
        e => bail!("Invalid fucking thing {}", e)
    };

    let whatthefuck = interaction.data.options.get(1)
        .context("No whatthefuck found")?
        .value
        .as_str().context("whatthefuck is not a str")?;

    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    let mut d = lc.db().await;
    let q = match whr {
        WhereTheFuck::Actions => {
            let q: Vec<(i64, Action)> = query_file!("src/data/query/search_actions.sql", whatthefuck)
                .fetch_all(&mut d).await?
                .into_iter()
                .map(|i| {
                    (i.id, Action {
                        target_id: i.target_id,
                        target_username: i.target_username,
                        offense: i.offense.expect("Should never be none"),
                        action: i.action.expect("Should never be none"),
                        server: i.server,
                        claimant: i.claimant,
                        report: i.report,
                    })
                }).collect();
            let action_count = sqlx::query!("select count(*) as \"count\" from ActionsSearch where ActionsSearch match ?;", whatthefuck)
                .fetch_one(&mut d).await?.count;
            Resultz::Actions((q, action_count))
        }
        WhereTheFuck::Reports => {
            let q: Vec<(i64, Report)> = query_file!("src/data/query/search_reports.sql", whatthefuck)
                .fetch_all(&mut d).await?
                .into_iter()
                .map(|i| {
                    (i.id, Report {
                        reporter_id: i.reporter_id,
                        reporter_name: i.reporter_name,
                        reported_id: i.reported_id,
                        reported_name: i.reported_name,
                        report_reason: i.report_reason.expect("Fuck you, this should never fail"),
                        report_status: i.report_status,
                        server: i.server,
                        time: i.time,
                        claimant: i.claimant,
                        audit: i.audit,
                    })
                }).collect();
            let report_count = sqlx::query!("select count(*) as \"count\" from ReportSearch where ReportSearch.report_reason match ?;", whatthefuck)
                .fetch_one(&mut d).await?.count;
            Resultz::Reports((q, report_count))
        }

    };
    // time to fucking create the embed
    let e = CreateEmbed::new().
        title(format!("Search results for '{}' in '{:?}'", whatthefuck, whr))
        .color(Color::BLURPLE)
        .timestamp(Timestamp::now());
    let embed = match q {
        Resultz::Actions((res, c)) => {
            e.description(res.into_iter().map(|a| {
                format!("* {} - `{}` -> `{}`\n", a.0, a.1.offense, a.1.action)
            }).collect::<String>())
                .footer(CreateEmbedFooter::new(format!("{} results total", c)))
        }
        Resultz::Reports((res, c)) => {
            e.description(res.into_iter().map(|a| {
                format!("* {} - `{}`\n", a.0, a.1.report_reason)
            }).collect::<String>()).footer(CreateEmbedFooter::new(format!("{} results total", c)))
        }
    };
    interaction.edit_response(&ctx, EditInteractionResponse::new().embeds(vec![embed])).await?;
    Ok(())
}

pub fn register() -> (CreateCommand, &'static str) {
    (CreateCommand::new("search")
         .description("Search the library!")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "where",
                "WHERE DO YOU WANT TO SEARCH",
            )
                .add_string_choice("Reports", "reports")
                .add_string_choice("Actions", "actions")
                .required(true)
        ).add_option(
        CreateCommandOption::new(CommandOptionType::String, "query", "WHAT DO YOU WANT TO KNOW?").required(true)
    ),
     "search")
}
