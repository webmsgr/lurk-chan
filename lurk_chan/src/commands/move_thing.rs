use common::Location;
use poise::{
    serenity_prelude::{ChannelId, CreateMessage, MessageId, Timestamp},
    CreateReply,
};

#[derive(poise::ChoiceParameter)]
enum Choices {
    SL,
    Discord,
}

impl Into<Location> for Choices {
    fn into(self) -> Location {
        match self {
            Choices::SL => Location::SL,
            Choices::Discord => Location::Discord,
        }
    }
}

#[poise::command(slash_command, rename = "move", subcommands("audit"))]
pub async fn move_command(_: crate::Context<'_>) -> anyhow::Result<()> {
    // no
    Ok(())
}

/// Move an audit to another server
#[poise::command(slash_command)]
async fn audit(
    ctx: crate::ApplicationContext<'_>,
    #[description = "Audit ID to move"] id: u32,
    #[description = "Where to move it"] location: Choices,
) -> anyhow::Result<()> {
    let location: Location = location.into();
    ctx.defer_ephemeral().await?;

    let current_audit = ctx.data().db.get_action_from_id(id).await?;

    if let Some(mut a) = current_audit {
        if ctx.author().id.get() != a.claimant {
            ctx.send(
                CreateReply::default()
                    .content("You can't move someone else's audit!")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
        if a.server == location {
            ctx.send(
                CreateReply::default()
                    .content("That audit is already there!")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
        // first, edit the audit
        a.server = location.clone();
        // get the old message
        let new_channel = match a.server {
            Location::SL => ctx.data().config.secret_lab.audit,
            Location::Discord => ctx.data().config.discord.audit,
        };
        let old_message = ctx.data().db.get_action_message(id).await?;
        if let Some((channel_id, message_id)) = old_message {
            let channel_id = ChannelId::new(channel_id);
            let message_id = MessageId::new(message_id);
            let old_message = channel_id
                .message(ctx.serenity_context(), message_id)
                .await?;

            // update db

            ctx.data()
                .db
                .edit_action(
                    id,
                    a.clone(),
                    Timestamp::now().to_string(),
                    ctx.author().id.get(),
                )
                .await?;

            // send a new message
            let new_message = new_channel
                .send_message(
                    ctx.serenity_context(),
                    CreateMessage::default()
                        .embed(
                            lurk_chan::create_action_embed(
                                &a,
                                &ctx.serenity_context(),
                                id,
                                new_channel,
                            )
                            .await?,
                        )
                        .components(lurk_chan::create_action_components(id)),
                )
                .await?;

            // remove old message
            old_message.delete(ctx.serenity_context()).await?;

            // update DB
            ctx.data()
                .db
                .add_action_message(new_message.channel_id.get(), new_message.id.get(), id)
                .await?;

            ctx.send(
                CreateReply::default()
                    .content(format!("Audit #{} moved to {:?}!", id, location))
                    .ephemeral(true),
            )
            .await?;
        } else {
            ctx.send(
                CreateReply::default()
                    .content(format!("Audit #{} not found!", id))
                    .ephemeral(true),
            )
            .await?;
        }
    } else {
        ctx.send(
            CreateReply::default()
                .content(format!("Audit #{} not found!", id))
                .ephemeral(true),
        )
        .await?;
    }

    Ok(())
}
