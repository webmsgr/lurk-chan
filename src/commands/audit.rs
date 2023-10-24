use crate::audit::Location;
use crate::prefabs::{audit_log_modal, AutofillAuditLog};
use serenity::all::{CommandInteraction, CommandOptionType};
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::prelude::*;
use std::collections::HashMap;

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<()> {
    let mut autofill_base = HashMap::with_capacity(5);
    for arg in interaction.data.options.iter() {
        autofill_base.insert(
            arg.name.as_str(),
            arg.value.as_str().expect("only str args"),
        );
    }

    let autofill = AutofillAuditLog {
        location: autofill_base
            .get("location")
            .map(|v| match *v {
                "Discord" => Location::Discord,
                "SL" => Location::SL,
                _ => unreachable!(),
            })
            .unwrap_or_default(),
        id: autofill_base.get("id").map(|v| v.to_string()),
        name: autofill_base.get("name").map(|v| v.to_string()),
        offense: autofill_base.get("offense").map(|v| v.to_string()),
        punishment: autofill_base.get("punishment").map(|v| v.to_string()),
    };

    interaction
        .create_response(ctx, audit_log_modal(None, 'r', Some(autofill)))
        .await?;
    //interaction.create_followup(ctx, CreateInteractionResponseFollowup::new().content("ok").ephemeral(true)).await?;
    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("audit")
        .description("Create a new audit log entry")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "location",
                "Where is the audit location",
            )
            .add_string_choice("Discord", "Discord")
            .add_string_choice("SL", "SL"),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "id",
            "ID of the audited user",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "name",
            "Name of the audited user",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "offense",
            "Offense of the audited user",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "punishment",
            "Punishment of the audited user",
        ))
}
