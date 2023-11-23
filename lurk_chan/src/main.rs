use std::sync::Arc;
use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    io::BufReader,
    path::PathBuf,
    time::Duration,
};
mod commands;
use anyhow::{anyhow, bail, Context as _, Error};
use async_shutdown::ShutdownManager;
use common::{Action, Location, Report};
use lurk_chan::{transmute_json, update_audit_message, update_report_message};
use poise::serenity_prelude::{ChannelId, Client, GuildId};
use poise::{
    execute_modal, execute_modal_on_component_interaction,
    serenity_prelude::{
        self, async_trait, ActivityData, ComponentInteraction, CreateInteractionResponse,
        CreateInteractionResponseFollowup, CreateInteractionResponseMessage, CreateMessage,
        EditInteractionResponse, EventHandler, FullEvent, Timestamp,
    },
    BoxFuture, Framework, FrameworkContext, FrameworkOptions, Modal,
};
use tracing::{error, info};
mod tasks;
use database::Database;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    main: MainConfig,
    secret_lab: SLConfig,
    discord: DiscordConfig,
}
#[derive(Deserialize)]
pub struct MainConfig {
    token: String,
}
#[derive(Deserialize)]
pub struct SLConfig {
    audit: ChannelId,
}
#[derive(Deserialize)]
pub struct DiscordConfig {
    audit: ChannelId,
    stats: ChannelId,
    debug_guild: Option<GuildId>,
}

pub const DEFAULT_CONFIG: &str = include_str!("../default_config.toml");

fn load_or_create_config() -> anyhow::Result<Config> {
    let config_path = PathBuf::from("config.toml");
    if !config_path.exists() {
        std::fs::write(&config_path, DEFAULT_CONFIG).context("Failed to create default config")?;
    }
    let config_file =
        String::from_utf8(std::fs::read(config_path).context("failed to read config file")?)
            .context("config file is not utf8!")?;
    Ok(toml::from_str(&config_file).context("Failed to parse config file")?)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Hello, world!");
    // load config
    let config = load_or_create_config()?;
    let shutdown = ShutdownManager::new();
    // validate token
    poise::serenity_prelude::validate_token(&config.main.token).context("Invalid token")?;

    setup_control_c(shutdown.clone());

    bot(config, Database::new().await?, shutdown.clone()).await?;

    let reason = shutdown.wait_shutdown_triggered().await;
    info!("Shutting down: {:?}", reason);

    let _ = shutdown.wait_shutdown_complete().await;
    info!("Goodbye!");
    Ok(())
}

fn setup_control_c(s: ShutdownManager<&'static str>) {
    tokio::task::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c");
        let _ = s.trigger_shutdown("ctrl-c");
    });
}

pub type Context<'a> = poise::Context<'a, LurkChan, anyhow::Error>;
pub type ApplicationContext<'a> = poise::ApplicationContext<'a, LurkChan, anyhow::Error>;

async fn handle(
    evt: &FullEvent,
    framework: FrameworkContext<'_, LurkChan, Error>,
) -> anyhow::Result<()> {
    match evt {
        FullEvent::Ready {
            ctx,
            data_about_bot,
        } => {
            info!(
                "And {} v{} takes the stage!",
                data_about_bot.user.name,
                env!("CARGO_PKG_VERSION")
            );
            ctx.set_activity(Some(ActivityData::watching(format!(
                "for new reports! (v{})",
                env!("CARGO_PKG_VERSION")
            ))));
        }
        FullEvent::Message { ctx, new_message } => {
            on_message(ctx, new_message, framework.user_data).await?;
        }
        FullEvent::InteractionCreate { ctx, interaction } => {
            if let Some(r) = interaction.as_message_component() {
                if let Err(e) = on_button(ctx, r, framework.user_data).await {
                    info!("Error handling button: {:?}", e);
                    r.create_response(
                        ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .content("Error handling button. yell at wackery please")
                                .ephemeral(true),
                        ),
                    )
                    .await?;
                    return Err(e);
                }
            }
        }
        _ => {}
    }
    Ok(())
}
#[derive(Modal, Default, Debug)]
#[name = "Audit Log"]
pub struct AuditModal {
    #[name = "ID"]
    #[placeholder = "ID of the user to audit"]
    pub id: String,
    #[name = "Name"]
    #[placeholder = "Name of the user to audit"]
    pub name: String,
    #[name = "Reason"]
    #[placeholder = "Reason for the audit"]
    pub reason: String,
    #[name = "Action"]
    #[placeholder = "What action to take"]
    pub action: String,
}

impl From<Action> for AuditModal {
    fn from(a: Action) -> Self {
        Self {
            id: a.target_id,
            name: a.target_username,
            reason: a.offense,
            action: a.action,
        }
    }
}

struct WhatTheFuck<'a>(&'a serenity_prelude::Context);

impl<'a> AsRef<serenity_prelude::Context> for WhatTheFuck<'a> {
    fn as_ref(&self) -> &serenity_prelude::Context {
        self.0
    }
}

pub async fn on_button(
    ctx: &serenity_prelude::Context,
    int: &ComponentInteraction,
    lc: &LurkChan,
) -> anyhow::Result<()> {
    // piss shit anmd die
    info!("Who the fuck touched the button?");
    let (kind, id) = int
        .data
        .custom_id
        .split_once('_')
        .expect("Invalid custom id, this should never fuckign happen");
    let id: u32 = id.parse().expect("Failed to parse id, fuck!");
    //let mut m = int.message.clone();
    //
    let uid = int.user.id.get();
    match kind {
        "claim" => {
            int.defer_ephemeral(ctx).await?;
            lc.db.claim_report(id, uid).await?;
            update_report_message(ctx, id, &lc.db).await?;
        }
        "close" => {
            let report = lc
                .db
                .get_report_from_id(id)
                .await?
                .context("That report dont exist")?;
            if report.claimant.is_some_and(|i| i == uid) {
                // fuck
                let resp = execute_modal_on_component_interaction(
                    WhatTheFuck(ctx),
                    Arc::new(int.clone()),
                    Some(AuditModal {
                        id: report.reported_id,
                        name: report.reported_name,
                        reason: report.report_reason,
                        ..Default::default()
                    }),
                    Some(Duration::from_secs(120)),
                )
                .await?;

                if resp.is_none() {
                    return Ok(());
                }
                let resp = resp.unwrap();

                // create an action from resp
                let a = Action {
                    target_id: resp.id,
                    target_username: resp.name,
                    offense: resp.reason,
                    action: resp.action,
                    server: report.location,
                    report: Some(id),
                    claimant: uid,
                };

                let channel_for_msg = match &a.server {
                    &Location::SL => lc.config.secret_lab.audit,
                    &Location::Discord => lc.config.discord.audit,
                };
                lc.db.close_report(id).await?;
                let aid = lc.db.add_action(a.clone()).await?;
                let m = channel_for_msg
                    .send_message(
                        ctx,
                        CreateMessage::default()
                            .embed(
                                lurk_chan::create_action_embed(&a, ctx, aid, channel_for_msg)
                                    .await?,
                            )
                            .components(lurk_chan::create_action_components(aid)),
                    )
                    .await?;
                lc.db
                    .add_action_message(m.channel_id.get(), m.id.get(), aid)
                    .await?;
                update_report_message(ctx, id, &lc.db).await?;
                int.create_followup(
                    ctx,
                    CreateInteractionResponseFollowup::default()
                        .content(":+1:")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            } else {
                // that doesnt fucking belong to you, dipshit
                int.create_followup(
                    ctx,
                    CreateInteractionResponseFollowup::default()
                        .content("sorry buddy, that doesn't belong to you")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }
        }
        "forceclose" => {
            int.defer_ephemeral(ctx).await?;
            let report = lc
                .db
                .get_report_from_id(id)
                .await?
                .context("That report dont exist")?;
            if report.claimant.is_some_and(|i| i == uid) {
                lc.db.close_report(id).await?;
                update_report_message(ctx, id, &lc.db).await?;
            }
        }
        "edit" => {
            let action = lc
                .db
                .get_action_from_id(id)
                .await?
                .context("That action do no exis")?;
            if action.claimant != uid {
                int.create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content("sorry buddy, that doesn't belong to you")
                            .ephemeral(true),
                    ),
                )
                .await?;
                return Ok(());
            }
            // lets fucking do this
            let resp = execute_modal_on_component_interaction(
                WhatTheFuck(ctx),
                Arc::new(int.clone()),
                Some(Into::<AuditModal>::into(action.clone())),
                Some(Duration::from_secs(120)),
            )
            .await?;
            if resp.is_none() {
                return Ok(());
            }
            let resp = resp.unwrap();
            let a = Action {
                target_id: resp.id,
                target_username: resp.name,
                offense: resp.reason,
                action: resp.action,
                ..action
            };

            lc.db
                .edit_action(id, a, Timestamp::now().to_string(), uid)
                .await?;
            update_audit_message(ctx, id, &lc.db).await?;
            int.create_followup(
                ctx,
                CreateInteractionResponseFollowup::default()
                    .content(":+1:")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
        e => {
            //error!("Invalid button type: {}", e);
            bail!("Invalid button type: {}", e);
        }
    }
    int.edit_response(ctx, EditInteractionResponse::default().content(":+1:"))
        .await?;
    Ok(())
}

pub fn report_from_msg(msg: &serenity_prelude::Message) -> anyhow::Result<Option<Report>> {
    if let Some(embed) = msg.embeds.get(0) {
        if embed.title.as_deref() != Some("Player Report") {
            return Ok(None);
        }
        // this is probably a report! yay!
        let mut field_ma = HashMap::with_capacity(embed.fields.len());
        for field in &embed.fields {
            field_ma.insert(field.name.clone(), field.value.replace('`', ""));
        }
        // transmute the field_ma into a Report
        //info!("{:#?}", field_ma);
        let r: Report = match transmute_json(field_ma) {
            Ok(v) => v,
            Err(err) => {
                return Err(err.into());
            }
        };
        return Ok(Some(r));
    }
    Ok(None)
}

async fn on_message(
    ctx: &serenity_prelude::Context,
    new_message: &serenity_prelude::Message,
    lc: &LurkChan,
) -> anyhow::Result<()> {
    if let Some(report) = report_from_msg(&new_message)? {
        // holy shit this is a report!
        // add that shit to the db
        let id = lc.db.add_report(report.clone()).await?;
        // send the report message
        let (embed, comp) = lurk_chan::create_things_from_report(report, id, &lc.db).await?;
        let m = new_message
            .channel_id
            .send_message(ctx, CreateMessage::default().embed(embed).components(comp))
            .await?;
        lc.db
            .add_report_message(m.channel_id.get(), m.id.get(), id)
            .await?;
        new_message.delete(ctx).await?;
        return Ok(());
    }
    Ok(())
}

pub struct LurkChan {
    pub config: Config,
    pub db: Database,
    pub shutdown: ShutdownManager<&'static str>,
}

async fn bot(config: Config, db: Database, s: ShutdownManager<&'static str>) -> anyhow::Result<()> {
    let framework_shutdown = s.clone();
    let background_shutdown = s.clone();
    use poise::serenity_prelude::GatewayIntents;
    let client = Client::builder(
        &config.main.token,
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .framework(Framework::new(
        FrameworkOptions {
            commands: commands::commands(),
            event_handler: |evt, ctx, _| {
                Box::pin(async move {
                    if let Err(e) = handle(evt, ctx).await {
                        println!("Error handling event ({:?}): {:?}", evt, e);
                        return Err(e);
                    }
                    Ok(())
                })
            },
            initialize_owners: true,
            skip_checks_for_owners: true,
            ..Default::default()
        },
        |ctx, _ready, framework: &Framework<LurkChan, anyhow::Error>| {
            Box::pin(async move {
                tasks::start_all_background_tasks(ctx.clone(), background_shutdown).await?;
                #[cfg(debug_assertions)]
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    config
                        .discord
                        .debug_guild
                        .context("No debug guild in debug mode!")?,
                )
                .await?;
                #[cfg(not(debug_assertions))]
                poise::builtins::register_globally(ctx, &framework.options().commands)
                    .await
                    .context("failed to register commands")?;
                Ok(LurkChan {
                    config,
                    db,
                    shutdown: framework_shutdown,
                })
            })
        },
    ))
    .await?;
    let client_shutdowner = s.clone();
    let client_wait_shutdowner = s.clone();
    let shard_man = client.shard_manager.clone();
    tokio::task::spawn(
        client_wait_shutdowner
            .wrap_delay_shutdown(async move {
                let _ = client_shutdowner.wait_shutdown_triggered().await;
                info!("Shutting down client");
                let _ = shard_man.shutdown_all().await;
                info!("Client shut down");
            })
            .expect("not already shutting down"),
    );
    tokio::task::spawn(async move {
        let mut client = client;

        if let Err(e) = client.start_autosharded().await {
            println!("Client error: {:?}", e);
        }
        let _ = s.trigger_shutdown("Client shutdown");
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{Config, DEFAULT_CONFIG};

    #[test]
    fn default_config_parses() {
        let _: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
    }
}
