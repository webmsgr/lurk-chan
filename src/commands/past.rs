use serenity::all::{CommandInteraction, CommandOptionType, CreateEmbed, Timestamp};
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse,
};
use serenity::prelude::*;
use sqlx::{query_as, Acquire};
use std::error::Error;
use std::future::Future;
use std::sync::Arc;

use crate::audit::Action;
use crate::audit::Location;
use crate::report::ReportStatus;
use crate::LurkChan;
use crate::Report;

pub fn run<'a>(
    ctx: &'a Context,
    interaction: &'a CommandInteraction,
) -> impl Future<Output = anyhow::Result<()>> + Send + 'a {
    async move {
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Defer(
                    CreateInteractionResponseMessage::default().ephemeral(true),
                ),
            )
            .await?;
        let lc = {
            let data = ctx.data.read().await;
            Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
        };
        let mut db = lc.db().await;
        // grab the first argument
        let id = interaction
            .data
            .options
            .get(0)
            .expect("Option to be present")
            .value
            .as_str()
            .expect("Value to be a string");
        let (reportee, reportee_count) = {
            (
                query_as!(Report, "select reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status as \"report_status: ReportStatus\", server, time, claimant, audit from Reports where reporter_id = ? order by time desc limit 5", id).fetch_all(&mut db).await?,
                sqlx::query!("select count(*) as \"count\" from Reports where reporter_id = ?", id).fetch_one(&mut db).await?.count
            )
        };
        let (reported, reported_count) = {
            (
                query_as!(Report, "select reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status as \"report_status: ReportStatus\", server, time, claimant, audit from Reports where reported_id = ? order by time desc limit 5", id).fetch_all(&mut db).await?,
                sqlx::query!("select count(*) as \"count\" from Reports where reporter_id = ?", id).fetch_one(&mut db).await?.count
            )
        };
        let (actions, actions_count) = {
            (
                query_as!(Action, "select target_id, target_username, offense, action, server as \"server: Location\", claimant, report from Actions where target_id = ? limit 5;", id).fetch_all(&mut db).await?,
                sqlx::query!("select count(*) as \"count\" from Actions where target_id = ?", id).fetch_one(&mut db).await?.count
            )
        };

        //generate the modal

        let modal: CreateEmbed = CreateEmbed::new()
        .title(format!("{}'s past...", id))
        .description(format!("{} has been:\n- Reported {} time(s)\n- Reported someone {} time(s)\n- Had action taken against them {} time(s)",id, reported_count, reportee_count, actions_count))
        .field("Recent reported", {
            reportee.into_iter().map(|i| {
                format!("- Reported by {} on <t:{}:f> for '{}' ({})\n", i.reporter_name, i.time.parse::<Timestamp>().expect("invalid timestamp").timestamp(), i.report_reason, i.report_status_string())
            }).collect::<String>()
        }, false)
        .field("Recent reports", {
            reported.into_iter().map(|i| {
                format!("- Reported {} on <t:{}:f> for '{}' ({})\n", i.reported_name, i.time.parse::<Timestamp>().expect("invalid timestamp").timestamp(), i.report_reason, i.report_status_string())
            }).collect::<String>()
        }, false)
        .field("Recent action", {
            actions.into_iter().map(|i| {
                format!("- '{}' for '{}' by <@!{}> ({})\n", i.action, i.offense, i.claimant, i.report.and_then(|o| Some(o.to_string())).unwrap_or_else(|| "No report".to_string()))
            }).collect::<String>()
        }, false);
        interaction
            .edit_response(&ctx, EditInteractionResponse::new().embed(modal))
            .await?;
        //info!("reportee: {:?}, reported: {:?}",reportee, reported);
        Ok(())
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("past")
        .description("View a user's past infractions")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "id",
                "Id in database, either discord id or <steamid>@steam",
            )
            .required(true),
        )
}
