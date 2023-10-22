use crate::audit::Location;
use crate::prefabs::audit_log_modal;
use crate::report::Report;
use serenity::all::{CommandInteraction, CommandOptionType, CommandType};
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::prelude::*;
use std::error::Error;

pub async fn run(
    ctx: &Context,
    interaction: &CommandInteraction,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (user, u) = interaction
        .data
        .resolved
        .users
        .iter()
        .next()
        .expect("Failed to get user");
    //let user = interaction.data.options.get(0).expect("a user to be provided").value.as_user_id().expect("a user to be a user");
    let nick = u.global_name.as_ref().unwrap_or_else(|| &u.name).clone();
    interaction
        .create_response(
            ctx,
            audit_log_modal(
                None,
                'r',
                Some(Report {
                    reported_id: user.get().to_string(),
                    reported_name: nick,
                    ..Default::default()
                }),
                Location::Discord,
                None,
            ),
        )
        .await?;
    //interaction.create_followup(ctx, CreateInteractionResponseFollowup::new().content("ok").ephemeral(true)).await?;
    Ok(())
}

pub mod user {
    use super::*;
    pub fn register() -> CreateCommand {
        CreateCommand::new("discord_audit")/*.description("Create a new audit log entry for a discord user")*/.kind(CommandType::User)
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
