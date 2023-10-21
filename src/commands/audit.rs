use crate::audit::Location;
use crate::prefabs::audit_log_modal;
use serenity::all::CommandInteraction;
use serenity::builder::CreateCommand;
use serenity::prelude::*;
use std::error::Error;

pub async fn run(
    ctx: &Context,
    interaction: &CommandInteraction,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    interaction
        .create_response(ctx, audit_log_modal(None, None, Location::SL))
        .await?;
    //interaction.create_followup(ctx, CreateInteractionResponseFollowup::new().content("ok").ephemeral(true)).await?;
    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("audit").description("Create a new audit log entry")
}
