mod audit;
mod db;
mod interactions;
mod prefabs;
mod report;
mod commands;
mod console;

use async_shutdown::ShutdownManager;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::report::Report;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::env::var;
use std::result::Result;
use std::sync::Arc;
use once_cell::sync::Lazy;
use serenity::builder::CreateMessage;
use serenity::gateway::ActivityData;
use tracing::{error, info, instrument};
use crate::audit::{DISC_AUDIT, SL_AUDIT};

/// this struct is passed around in an arc to share state
pub struct LurkChan {
    pub db: SqlitePool,
}

impl TypeMapKey for LurkChan {
    type Value = Arc<LurkChan>;
}

#[tokio::main]
async fn main() {
    color_backtrace::install();
    tracing_subscriber::fmt::init();
    let _ = dotenvy::dotenv();
    Lazy::force(&SL_AUDIT);
    Lazy::force(&DISC_AUDIT);
    info!("Hello, world!");
    // connect to the database
    //let db_url = var("DATABASE_URL").unwrap_or_else(|| "sqlite://lurk_chan.db".to_string());
    let options = SqliteConnectOptions::default()
        .foreign_keys(true)
        .create_if_missing(true)
        .filename("lurk_chan.db");
    let db = SqlitePool::connect_with(options)
        .await
        .expect("Failed to connect to database!");
    sqlx::migrate!()
        .run(&db)
        .await
        .expect("Failed to run migrations, database is fucked!");
    let lurk_chan = Arc::new(LurkChan { db });

    let shutdown = ShutdownManager::new();

    // thbanks docs
    tokio::spawn({
        let shutdown = shutdown.clone();
        async move {
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!("Failed to wait for CTRL+C: {}", e);
                std::process::exit(1);
            } else {
                shutdown.trigger_shutdown("Control-C").ok();
            }
        }
    });
    console::spawn_console(shutdown.clone());
    let token = var("DISCORD_TOKEN").expect("token");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .type_map_insert::<LurkChan>(lurk_chan)
        .event_handler(Handler)
        .await
        .expect("Failed to create client");
    let ctx = (client.cache.clone(), client.http.clone());
    let shard_manager = client.shard_manager.clone();
    tokio::task::spawn(shutdown.wrap_trigger_shutdown("Client died", async move {
        if let Err(c) = client.start_autosharded().await {
            error!("Error running client {:?}", c);
        }
    }));
    let shutdown_future = shutdown.wait_shutdown_triggered();
    let shard_shutdown_future = shutdown_future.clone();
    let _ = shutdown
        .wrap_delay_shutdown(async move {
            shard_shutdown_future.await;
            shard_manager.shutdown_all().await;
        })
        .expect("failed to do the thing");
    tokio::task::spawn(async move {
        let new_ctx = (&ctx.0, ctx.1.http());
        while ctx.1.application_id().is_none() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        info!("Creating commands...");
        commands::register_commands(&new_ctx).await;
    });
    let reason = shutdown_future.await;
    info!("Shutting down! Reason: {}", reason);

    shutdown.wait_shutdown_complete().await;

    info!("Thank you for your service");
}

struct Handler;

pub fn report_from_msg(
    msg: &Message,
) -> Result<Option<Report>, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(embed) = msg.embeds.get(0) {
        if embed.title.as_deref() != Some("Player Report") {
            return Ok(None);
        }
        // this is probably a report! yay!
        let mut field_ma = HashMap::with_capacity(embed.fields.len());
        for field in &embed.fields {
            field_ma.insert(field.name.clone(), field.value.replace("`", ""));
        }
        // transmute the field_ma into a Report
        //info!("{:#?}", field_ma);
        let r: Report = match serde_json::to_value(field_ma).and_then(|v| serde_json::from_value(v))
        {
            Ok(v) => v,
            Err(err) => {
                return Err(err.into());
            }
        };
        return Ok(Some(r));
    }
    Ok(None)
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, data_about_bot: Ready) {
        info!(
            "{} v{} is ready for action!",
            data_about_bot.user.name,
            env!("CARGO_PKG_VERSION")
        );
        ctx.set_activity(Some(ActivityData::watching("for new reports!")));
        
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        interactions::on_interaction(ctx, interaction).await;
    }
    #[instrument(skip(ctx, new_message, self))]
    async fn message(&self, ctx: Context, new_message: Message) {
        let r = match report_from_msg(&new_message) {
            Ok(Some(r)) => r,
            Err(e) => {
                error!("Error parsing report: {}", e);
                return;
            }
            _ => return,
        };
        let msg_report = r.clone();
        // insert the report in the database
        let lc = {
            let data = ctx.data.read().await;
            Arc::clone(data.get::<LurkChan>().expect("Failed to get lurk_chan"))
        };
        let q = db::add_report(r, &lc.db).await;
        let id = match q {
            Ok(res) => res.last_insert_rowid(),
            Err(err) => {
                error!("Error inserting cheater report: {}", err);
                return;
            }
        };

        // send a whole new message
        let comp = msg_report.components(id);
        if let Err(e) = new_message
            .channel_id
            .send_message(&ctx, CreateMessage::default().embed(msg_report.create_embed()).components(comp))
            .await
        {
            error!("Failed to send new messgae: {}", e);
            return;
        }

        // delete the old message
        if let Err(e) = new_message.delete(&ctx).await {
            error!("Failed to delete old message: {}", e);
            return;
        }
    }
}
