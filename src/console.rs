use std::io::{stdin, IsTerminal};
use std::sync::Arc;
use std::thread;

use async_shutdown::ShutdownManager;

use sqlx::Acquire;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{error, info, instrument};

#[instrument(skip(s, lc))]
pub fn spawn_console(s: ShutdownManager<&'static str>, lc: Arc<LurkChan>) {
    let dead_token = s.trigger_shutdown_token("Console thread died.");
    let (tx, rx) = unbounded_channel();
    thread::spawn(move || {
        console_thread(tx);
        info!("Shuttdown triggered because read_console thread fucking died");
        drop(dead_token); //t
    });
    let s2 = s.clone();
    tokio::task::spawn(async move {
        s.wrap_delay_shutdown(console_process(s2, rx, lc))
            .expect("failed to create console task").await
    });
}
#[instrument(skip(tx))]
fn console_thread(tx: UnboundedSender<String>) {
    if !stdin().is_terminal() {
        info!("We are not in a terminal, no console will be used");
        loop {}
    }
    loop {
        let mut input = String::new();
        while let Ok(_) = stdin().read_line(&mut input) {
            let i = std::mem::take(&mut input);
            if !tx.is_closed() {
                tx.send(i).expect("to not be closed");
            } else {
                break;
            }
        }
    }
}
#[instrument(skip(s, rx, lc))]
async fn console_process(s: ShutdownManager<&'static str>, mut rx: UnboundedReceiver<String>, lc: Arc<LurkChan>) {
    loop {
        tokio::select! {
            _ = s.wait_shutdown_triggered() => {
                info!("Console task shutting down!");
                break;
            }
            Some(msg) = rx.recv() => {
                //info!("{}", msg);
                let msg = match shellwords::split(&msg) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("Error parsing command: \n{}", e);
                        continue;
                    }
                };

                match Command::try_parse_from(msg).and_then(|e| Ok(e.command.clone())) {
                    Ok(Commands::Quit) => {
                        let _ = s.trigger_shutdown("Console request");
                    },
                    Ok(Commands::Health) => {
                        // query the db
                        let mut db = lc.db().await;
                        let data: Result<(i64, i64, i64, i64, usize, bool), sqlx::Error> = async move {
                            let report_count = sqlx::query!("select count(*) as \"count: i64\" from Reports").fetch_one(db.acquire().await?).await?.count;
                            let action_count = sqlx::query!("select count(*) as \"count: i64\" from Actions").fetch_one(db.acquire().await?).await?.count;
                            let report_message_count = sqlx::query!("select count(*) as \"count: i64\" from ReportMessages").fetch_one(db.acquire().await?).await?.count;
                            let action_message_count = sqlx::query!("select count(*) as \"count: i64\" from ActionMessages").fetch_one(db.acquire().await?).await?.count;
                            let invalid_keys = sqlx::query!("PRAGMA foreign_key_check").fetch_all(db.acquire().await?).await?.len();
                            let integrety_check = sqlx::query!("PRAGMA integrity_check").fetch_one(db.acquire().await?).await?.integrity_check == "ok";
                           //let audit_message_count = sqlx::query!("select count(*) as \"count: i64\" from ").fetch_one(db).await.unwrap().count;
                            Ok((report_count, action_count, report_message_count, action_message_count, invalid_keys, integrety_check))
                        }.await;
                        match data {
                            Ok((report_count, action_count, report_message_count, action_message_count, invalid_keys, integrety_check)) => {
                                let is_db_healthy = invalid_keys == 0 && integrety_check;
                                info!("DB Health: ");
                                info!("\tReport Count: {}", report_count);
                                info!("\tAction Count: {}", action_count);
                                info!("\tReport Message Count: {}", report_message_count);
                                info!("\tAction Message Count: {}", action_message_count);
                                if invalid_keys > 0 {
                                    error!("\tThere are foreign key violations!");
                                } else {
                                    info!("\tNo foreign key violations detected.");
                                }
                                if !integrety_check {
                                    error!("\tIntegrety check failed!");
                                } else {
                                    info!("\tIntegrety check passed.");
                                }
                                if is_db_healthy {
                                    info!("DB is healthy!");
                                } else {
                                    error!("DB is not healthy!");
                                }
                            },
                            Err(e) => {
                                error!("Error getting DB Health: {}", e);
                            }
                        }
                        
                    },
                    Err(e) => {
                        let is_err = e.use_stderr();
                        let e = e.render();
                        e.to_string().split('\n').for_each(|l| if is_err { error!("{}", l) } else { info!("{}", l)});
                    }
                }
            }
        }
    }
}

use clap::{Parser, Subcommand};

use crate::LurkChan;

#[derive(Parser, Debug)]
#[command(no_binary_name(true), disable_help_flag(true))]
struct Command {
    #[clap(subcommand)]
    command: Commands,
}
#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Exits the bot
    Quit,

    /// Gets DB Health
    Health
}
