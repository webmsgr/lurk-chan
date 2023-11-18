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
                    .content("If you are seeing this message, then you have somehow bypassed the permission check, godspeed."),
            ),
        )
        .await?;
    Ok(())
}

pub fn register() -> (CreateCommand, &'static str) {
    (
        CreateCommand::new("when").description("when").add_option(
            CreateCommandOption::new(CommandOptionType::SubCommandGroup, "day", "day")
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "breaks",
                    "Golden days. In the sunshine of a happy youth",
                )),
        ),
        "when",
    )
}
