use std::error::Error;
use std::future::Future;
use std::sync::Arc;
use serenity::all::{CommandInteraction, CommandOptionType, CreateEmbed, Timestamp};
use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse};
use serenity::prelude::*;
use sqlx::query_as;

use crate::LurkChan;
use crate::report::ReportStatus;
use crate::Report;
use crate::audit::Action;
use crate::audit::Location;

pub fn run<'a>(ctx: &'a Context, interaction: &'a CommandInteraction) -> impl Future<Output = Result<(), Box<dyn Error + Send + Sync + 'a>>> + Send + 'a {
    async move {
        interaction.create_response(ctx, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::default().ephemeral(true))).await?;
    let lc = {
        let data = ctx.data.read().await;
        Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
    };
    let db = &lc.db;
    // grab the first argument
    let id = interaction.data.options.get(0).expect("Option to be present").value.as_str().expect("Value to be a string");
    let reportee = { query_as!(Report, "select reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status as \"report_status: ReportStatus\", server, time, claimant, audit from Reports where reporter_id = ? order by time desc", id).fetch_all(db).await }?;
    let reported = { query_as!(Report, "select reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status as \"report_status: ReportStatus\", server, time, claimant, audit from Reports where reported_id = ? order by time desc", id).fetch_all(db).await }?;
    let actions =  { query_as!(Action, "select target_id, target_username, offense, action, server as \"server: Location\", claimant, report from Actions where target_id = ?;", id).fetch_all(db).await }?;
    let last_reportee: Vec<&Report>  = reportee.iter().take(5).collect();
    let last_reported: Vec<&Report> = reported.iter().take(5).collect();
    let last_actions: Vec<&Action> = actions.iter().take(5).collect();

    //generate the modal

    let modal: CreateEmbed = CreateEmbed::new()
        .title(format!("{}'s past...", id))
        .description(format!("{} has been:\n- Reported {} time(s)\n- Reported someone {} time(s)\n- Had action taken against them {} time(s)",id, reported.len(), reportee.len(), actions.len()))
        .field("Recent reported", {
            last_reported.into_iter().map(|i| {
                format!("- Reported by {} on <t:{}:f> for '{}' ({})\n", i.reporter_name, i.time.parse::<Timestamp>().expect("invalid timestamp").timestamp(), i.report_reason, i.report_status_string())
            }).collect::<String>()
        }, false)
        .field("Recent reports", {
            last_reportee.into_iter().map(|i| {
                format!("- Reported {} on <t:{}:f> for '{}' ({})\n", i.reported_name, i.time.parse::<Timestamp>().expect("invalid timestamp").timestamp(), i.report_reason, i.report_status_string())
            }).collect::<String>()
        }, false)
        .field("Recent action", {
            last_actions.into_iter().map(|i| {
                format!("- '{}' for '{}' by <@!{}> ({})\n", i.action, i.offense, i.claimant, i.report.and_then(|o| Some(o.to_string())).unwrap_or_else(|| "No report".to_string()))
            }).collect::<String>()
        }, false);
    interaction.edit_response(&ctx, EditInteractionResponse::new().embed(modal)).await?;
    //info!("reportee: {:?}, reported: {:?}",reportee, reported);
    Ok(())
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("past").description("View a user's past infractions").add_option(
        CreateCommandOption::new(CommandOptionType::String, "id", "Id in database, either discord id or <steamid>@steam").required(true)
    )
}