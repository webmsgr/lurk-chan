use serenity::all::{CommandInteraction, CommandOptionType};
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::prelude::*;

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<()> {
    interaction
        .create_response(
            &ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("Sorry buddy, only god can run this command."),
            ),
        )
        .await?;
    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("judgement")
        .description("judgement")
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "day",
            "JUDGEMENT DAY",
        ))
}
