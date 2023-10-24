use serenity::all::CommandInteraction;
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::prelude::*;
use std::error::Error;

pub async fn run(
    ctx: &Context,
    interaction: &CommandInteraction,
) -> anyhow::Result<()> {
    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content({
                format!("Hello! I'm Lurk-chan v{}! Pnog!", env!("CARGO_PKG_VERSION"))
            })),
        )
        .await?;
    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("ping").description("Ping!")
}
