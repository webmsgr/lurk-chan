use std::path::PathBuf;
use std::sync::Arc;
mod commands;
use anyhow::Context as _;
use async_shutdown::ShutdownManager;
use common::Action;
use poise::serenity_prelude::{ChannelId, Client, GuildId};
use poise::{CreateReply, FrameworkError};
use poise::{Framework, FrameworkOptions, Modal};
use tracing::info;
mod tasks;
use database::Database;
use serde::Deserialize;
mod event;
#[derive(Deserialize, Clone)]
pub struct Config {
    main: MainConfig,
    secret_lab: SLConfig,
    discord: DiscordConfig,
}
#[derive(Deserialize, Clone)]
pub struct MainConfig {
    token: String,
}
#[derive(Deserialize, Clone)]
pub struct SLConfig {
    audit: ChannelId,
}
#[derive(Deserialize, Clone)]
pub struct DiscordConfig {
    reports: ChannelId,
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
    toml::from_str(&config_file).context("Failed to parse config file")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    color_backtrace::install();
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
pub struct LurkChan {
    pub config: Config,
    pub db: Arc<Database>,
    pub shutdown: ShutdownManager<&'static str>,
}

impl Clone for LurkChan {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            db: Arc::clone(&self.db),
            shutdown: self.shutdown.clone(),
        }
    }
}

async fn bot(config: Config, db: Database, s: ShutdownManager<&'static str>) -> anyhow::Result<()> {
    let framework_shutdown = s.clone();
    //let background_shutdown = s.clone();
    use poise::serenity_prelude::GatewayIntents;
    let client = Client::builder(
        &config.main.token,
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .framework(Framework::new(
        FrameworkOptions {
            commands: commands::commands(),
            event_handler: |ctx, evt, framework, _| {
                Box::pin(async move {
                    if let Err(e) = event::handle(ctx, evt, framework).await {
                        println!("Error handling event ({:?}): {:?}", evt, e);
                        return Err(e);
                    }
                    Ok(())
                })
            },
            initialize_owners: true,
            skip_checks_for_owners: true,
            on_error: |error| {
                Box::pin(async move {
                    if let FrameworkError::Command { ctx, error, .. } = error {
                        tracing::error!("Error in command: {}", error);
                        if let Err(e) = ctx
                            .send(
                                CreateReply::default()
                                    .content("Error! contact wackery!")
                                    .ephemeral(true),
                            )
                            .await
                        {
                            tracing::error!("Error while handling error: {}", e);
                        }
                    } else if let Err(e) = poise::builtins::on_error(error).await {
                        tracing::error!("Error while handling error: {}", e);
                    }
                })
            },
            ..Default::default()
        },
        |ctx, _ready, framework: &Framework<LurkChan, anyhow::Error>| {
            Box::pin(async move {
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
                let lc = LurkChan {
                    config,
                    db: Arc::new(db),
                    shutdown: framework_shutdown,
                };
                Ok(lc)
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
