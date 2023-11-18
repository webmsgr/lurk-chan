use serenity::all::{CommandInteraction, CommandType};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::prelude::*;

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<()> {
    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("nah")
                    .ephemeral(true),
            ),
        )
        .await?;
    Ok(())
}

pub fn register() -> (CreateCommand, &'static str) {
    (
        CreateCommand::new("Report Message").kind(CommandType::Message),
        "Report Message",
    )
}
