use anyhow::bail;
use std::collections::HashMap;
use std::sync::Arc;

use crate::audit::Location;
use crate::report::{ReportStatus, Report};
use crate::LurkChan;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::all::{
    ChannelId, CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommandOption,
    CreateEmbed, CreateEmbedFooter, EditInteractionResponse, MessageId, UserId,
};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};

use serenity::model::Timestamp;
use serenity::prelude::*;
use sqlx::{Either, Execute, Row};
use tracing::info;

enum QueryType {
    Reports,
    Actions,
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<()> {
    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::default().ephemeral(true),
            ),
        )
        .await?;
    let mut die = HashMap::with_capacity(interaction.data.options.len());
    let (sub, args) = match &interaction.data.options[0].value {
        CommandDataOptionValue::SubCommand(options) => (
            match interaction.data.options[0].name.as_str() {
                "reports" => QueryType::Reports,
                "actions" => QueryType::Actions,
                _ => bail!("Invalid subcommand"),
            },
            options,
        ),
        _ => bail!("Invalid subcommand"),
    };
    for option in args.iter().cloned() {
        //info!("{}: {:?}", option.name, option.value);
        match option.value {
            CommandDataOptionValue::String(s) => {
                die.insert(option.name.clone(), Value::String(s));
            }
            CommandDataOptionValue::User(u) => {
                die.insert(option.name.clone(), serde_json::to_value(u)?);
            }
            _ => bail!("Invalid subcommand option value"),
        }
    }
    let v = serde_json::to_value(&die)?;
    let q_args = match sub {
        QueryType::Reports => QueryArgs::Report(serde_json::from_value(v)?),
        QueryType::Actions => QueryArgs::Action(serde_json::from_value(v)?),
    };

    let mut args = vec![];
    let mut query = match &q_args {
        QueryArgs::Report(r_args) => {
            let mut base = "select reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status as \"report_status: ReportStatus\", server, time, claimant, audit, id, channel, message from Reports left join ReportMessages RM on Reports.id = RM.report_id".to_string();
            let mut args_str = vec![];
            if let Some(status) = &r_args.status {
                args_str.push("report_status = ?");
                args.push(status.to_string());
            }
            if let Some(claimant) = &r_args.claimant {
                args_str.push("claimant = ?");
                args.push(claimant.get().to_string());
            }
            if let Some(reporter_id) = &r_args.reporter_id {
                args_str.push("reporter_id = ?");
                args.push(reporter_id.to_string());
            }
            if let Some(reported_id) = &r_args.reported_id {
                args_str.push("reported_id = ?");
                args.push(reported_id.to_string());
            }
            if !args_str.is_empty() {
                base.push_str(" where ");
                base.push_str(&args_str.join(" and "));
            }
            base
        }
        QueryArgs::Action(r_args) => {
            let mut base = "select target_id, target_username, offense, action, server as \"server: Location\", claimant, report, id, channel, message from Actions left join ActionMessages on Actions.id = ActionMessages.action_id".to_string();
            let mut args_str = vec![];
            if let Some(target_id) = &r_args.target_id {
                args_str.push("target_id = ?");
                args.push(target_id.to_string());
            }
            if let Some(claimant) = &r_args.claimant {
                args_str.push("claimant = ?");
                args.push(claimant.get().to_string());
            }
            if let Some(server) = &r_args.server {
                args_str.push("server = ?");
                args.push(server.to_string());
            }
            if !args_str.is_empty() {
                base.push_str(" where ");
                base.push_str(&args_str.join(" and "));
            }
            base
        }
    };
    query.push_str(" order by id desc;");
    let mut q = sqlx::query(&query);
    for arg in args {
        q = q.bind(arg);
    }
    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    let mut db = lc.db().await;
    info!("query: {:?}", q.sql());
    let mut rows = q.fetch_many(&mut db).boxed();
    const RESULTS_COUNT: usize = 10;
    let mut results = Vec::with_capacity(RESULTS_COUNT);
    let mut r_count = 0;
    while let Some(row) = rows.try_next().await? {
        match row {
            Either::Left(res) => {
                info!("{:?}", res)
            }
            Either::Right(row) => {
                r_count += 1;
                if results.len() < RESULTS_COUNT {
                    results.push(row)
                }
            }
        }
    }
    use std::fmt::Write;
    let r_len = results.len();
    // im going to be honest, this is some of the worst code i have ever written. What in the actual fuck.
    let res_rows = results.into_iter().fold(String::new(), |mut o, i| {
        match sub {
            QueryType::Reports => {
                // reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status as \"report_status: ReportStatus\", server, time, claimant, audit, id, channel, message
                let channel = i
                    .get::<Option<String>, usize>(11)
                    .and_then(|i| i.parse().ok());
                let message = i
                    .get::<Option<String>, usize>(12)
                    .and_then(|i| i.parse().ok());
                let link = if let (Some(channel), Some(message)) = (channel, message) {
                    MessageId::new(message).link(ChannelId::new(channel), None)
                } else {
                    "No report message".to_string()
                };
                let status = Report {
                    report_status: i.get::<ReportStatus, usize>(5),
                    claimant: i.get(8),
                    audit: i.get(9),
                    ..Default::default()
                };
                let _ = writeln!(
                    o,
                    "* {}: {} (`{}`) reported {} (`{}`) for '{}' ({}) ({})",
                    i.get::<i32, usize>(10),
                    i.get::<String, usize>(1),
                    i.get::<String, usize>(0),
                    i.get::<String, usize>(3),
                    i.get::<String, usize>(2),
                    i.get::<String, usize>(4),
                    status.report_status_string(),
                    link
                );
                o
            }
            QueryType::Actions => {
                // target_id, target_username, offense, action, server as \"server: Location\", claimant, report, id, channel, message
                let channel = i
                    .get::<Option<String>, usize>(8)
                    .and_then(|i| i.parse().ok());
                let message = i
                    .get::<Option<String>, usize>(9)
                    .and_then(|i| i.parse().ok());
                let link = if let (Some(channel), Some(message)) = (channel, message) {
                    MessageId::new(message).link(ChannelId::new(channel), None)
                } else {
                    "No action message".to_string()
                };
                let _ = writeln!(
                    o,
                    "* {}: {} (`{}`) was '{}' on {:?} for '{}' by <@!{}> ({})",
                    i.get::<i32, usize>(7),
                    i.get::<String, usize>(1),
                    i.get::<String, usize>(0),
                    i.get::<String, usize>(3),
                    i.get::<Location, usize>(4),
                    i.get::<String, usize>(2),
                    i.get::<String, usize>(5),
                    link
                );
                o
            }
        }
    });
    info!(
        "{} results and the rows are {} size from {}",
        r_count,
        res_rows.len(),
        r_len
    );
    let output_enum = CreateEmbed::new()
        .title("Query results:")
        .description(res_rows)
        .footer(CreateEmbedFooter::new(format!("{} results", r_count)))
        .timestamp(Timestamp::now());

    interaction
        .edit_response(&ctx, EditInteractionResponse::new().embed(output_enum))
        .await?;
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
enum QueryArgs {
    Report(ReportArgs),
    Action(ActionArgs),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReportArgs {
    status: Option<ReportStatus>,
    claimant: Option<UserId>,
    reporter_id: Option<String>,
    reported_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionArgs {
    target_id: Option<String>,
    claimant: Option<UserId>,
    server: Option<Location>,
}

pub fn register() -> (CreateCommand, &'static str) {
    (
        CreateCommand::new("query")
            .description("Query for shit!")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "reports",
                    "Query for reports!",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "status", "Status")
                        .add_string_choice("Open", "Open")
                        .add_string_choice("Closed", "Closed")
                        .add_string_choice("Claimed", "Claimed")
                        .add_string_choice("Expired", "Expired"),
                )
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::User,
                    "claimant",
                    "Claimant",
                ))
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::String,
                    "reporter_id",
                    "Reporter",
                ))
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::String,
                    "reported_id",
                    "Reported",
                )),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "actions",
                    "Query for actions!",
                )
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::String,
                    "target_id",
                    "Target",
                ))
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::User,
                    "claimant",
                    "Claimant",
                ))
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "server", "Server")
                        .add_string_choice("Discord", "Discord")
                        .add_string_choice("SL", "SL"),
                ),
            ),
        "query",
    )
}
