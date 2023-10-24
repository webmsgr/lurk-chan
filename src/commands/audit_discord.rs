use crate::audit::Location;
use crate::prefabs::{audit_log_modal, AutofillAuditLog};
use crate::report::Report;
use serenity::all::{CommandInteraction, CommandOptionType, CommandType};
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::prelude::*;
use std::error::Error;

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<()> {
    let (user, u, message) = if let Some(i) = interaction.data.resolved.users.iter().next() {
        (i.0, i.1, None)
    } else {
        interaction
            .data
            .resolved
            .messages
            .iter()
            .next()
            .map(|m| (&m.1.author.id, &m.1.author, Some(m.1)))
            .expect("If no user, then message")
    };
    //let user = interaction.data.options.get(0).expect("a user to be provided").value.as_user_id().expect("a user to be a user");
    let nick = u.global_name.as_ref().unwrap_or_else(|| &u.name).clone();
    interaction
        .create_response(
            ctx,
            audit_log_modal(
                None,
                'r',
                Some(AutofillAuditLog {
                    location: Location::Discord,
                    id: Some(user.to_string()),
                    name: Some(nick),
                    offense: message.map(|m| m.link()),
                    ..Default::default()
                }),
            ),
        )
        .await?;
    //interaction.create_followup(ctx, CreateInteractionResponseFollowup::new().content("ok").ephemeral(true)).await?;
    Ok(())
}

pub mod user {
    use super::*;
    pub fn register() -> CreateCommand {
        CreateCommand::new("Audit User")/*.description("Create a new audit log entry for a discord user")*/.kind(CommandType::User)
    }
}

pub mod message {
    use super::*;
    pub fn register() -> CreateCommand {
        CreateCommand::new("Audit Message")/*.description("Create a new audit log entry for a discord user")*/.kind(CommandType::Message)
    }
}

pub mod slash {
    use super::*;
    pub fn register() -> CreateCommand {
        CreateCommand::new("discord")
            .description("Create a new audit log entry for a discord user")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "User to audit")
                    .required(true),
            )
    }
}
