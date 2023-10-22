mod audit;
mod commands;
mod console;
mod db;
mod interactions;
mod prefabs;
mod report;

use async_shutdown::ShutdownManager;

use db::add_report_message;
use serenity::async_trait;
use serenity::model::prelude::*;
use serenity::prelude::*;
use tokio::fs::DirEntry;
use tokio::select;

use crate::audit::{DISC_AUDIT, SL_AUDIT};
use crate::report::Report;
use once_cell::sync::Lazy;
use serenity::builder::CreateMessage;
use serenity::gateway::ActivityData;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::env::var;
use std::path::PathBuf;
use std::result::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, instrument};

/// this struct is passed around in an arc to share state
mod lc;
pub use lc::LurkChan;

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
        .optimize_on_close(true, None)
        .filename("lurk_chan.db");
    let db = SqlitePoolOptions::new()
        .min_connections(10)
        .max_connections(100)
        .connect_lazy_with(options);

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("Failed to run migrations, database is fucked!");
    let lurk_chan = Arc::new(LurkChan::new(db));

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
    tokio::task::spawn(
        shutdown
            .wrap_delay_shutdown(optimize_db_task(Arc::clone(&lurk_chan), shutdown.clone()))
            .expect("we are not already shutting down"),
    );
    tokio::task::spawn(
        shutdown
            .wrap_delay_shutdown(backup_task(Arc::clone(&lurk_chan), shutdown.clone()))
            .expect("we are not already shutting down"),
    );
    console::spawn_console(shutdown.clone(), Arc::clone(&lurk_chan));
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

#[instrument(skip(lc, s))]
async fn optimize_db_task(lc: Arc<LurkChan>, s: ShutdownManager<&'static str>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut db = lc.db().await;
    loop {
        select! {
            _ = interval.tick() => {},
            _ = s.wait_shutdown_triggered() => {
                break;
            }
        }
        info!("Optimizing DB");
        if let Err(e) = sqlx::query("PRAGMA optimize;").execute(&mut db).await {
            error!("Failed to optimize DB: {}!", e)
        } else {
            info!("DB optimized");
        }
    }
}

#[instrument(skip(lc, s))]
async fn backup_task(lc: Arc<LurkChan>, s: ShutdownManager<&'static str>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut db = lc.db().await;
    let backup_folder = PathBuf::from(".").join("backups");
    if !backup_folder.exists() {
        if let Err(e) = tokio::fs::create_dir(&backup_folder).await {
            error!("Failed to create backups directory: {}", e);
            return;
        }
    }
    loop {
        select! {
            _ = interval.tick() => {},
            _ = s.wait_shutdown_triggered() => {
                break;
            }
        }
        info!("Backing up DB");
        let now = Timestamp::now();
        let backup_file = backup_folder.join(format!("backup_{}.db", now.timestamp()));
        if let Err(e) = sqlx::query(&format!(
            "vacuum into '{}';",
            backup_file.to_str().expect("path")
        ))
        .execute(&mut db)
        .await
        {
            error!("Failed to backup the DB: {}! this is probably an issue!", e);
        }
        if let Ok(mut rd) = tokio::fs::read_dir("backups").await {
            let mut items = Vec::with_capacity(24);
            while let Ok(Some(i)) = rd.next_entry().await {
                items.push(i)
            }
            items.sort_by_cached_key(|v| v.file_name());
            if items.len() > 24 {
                let oldest = items[0].file_name();
                if let Err(e) = tokio::fs::remove_file(oldest).await {
                    error!("Failed to remove oldest backup: {}", e);
                }
                info!("Removed oldest backup")
            }
        }
        info!("DB backed up")
    }
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
        ctx.set_activity(Some(ActivityData::watching(format!(
            "for new reports! (v{})",
            env!("CARGO_PKG_VERSION")
        ))));
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
        let mut db = lc.db().await;
        let q = db::add_report(r, &mut db).await;
        let id = match q {
            Ok(res) => res.last_insert_rowid(),
            Err(err) => {
                error!("Error inserting cheater report: {}", err);
                return;
            }
        };

        // send a whole new message
        let comp = msg_report.components(id);
        let m = new_message
            .channel_id
            .send_message(
                &ctx,
                CreateMessage::default()
                    .embed(msg_report.create_embed(id))
                    .components(comp),
            )
            .await;
        let m = match m {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to send new messgae: {}", e);
                return;
            }
        };

        // insert the message

        if !add_report_message(id, m, &mut db).await {
            error!("Failed to insert message into database, this isn't fatal but its bad!");
        }

        // delete the old message
        if let Err(e) = new_message.delete(&ctx).await {
            error!("Failed to delete old message: {}", e);
            return;
        }
    }
}
