use std::io::{stdin, IsTerminal};
use std::thread;
use std::time::Duration;

use async_shutdown::ShutdownManager;

use poise::serenity_prelude::CacheHttp;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{error, info, instrument};

#[instrument(skip(s, lc, ctx))]
pub async fn console_task(
    ctx: impl CacheHttp,
    lc: LurkChan,
    s: ShutdownManager<&'static str>,
) -> anyhow::Result<()> {
    let dead_token = s.trigger_shutdown_token("Console thread died.");
    let (tx, rx) = unbounded_channel();
    thread::spawn(move || {
        console_thread(tx);
        info!("Shuttdown triggered because read_console thread fucking died");
        drop(dead_token); //t
    });
    console_process(s, rx, lc, ctx).await;
    Ok(())
}
#[instrument(skip(tx))]
fn console_thread(tx: UnboundedSender<String>) {
    if !stdin().is_terminal() {
        info!("We are not in a terminal, no console will be used");
        loop {
            thread::sleep(Duration::from_micros(u64::MAX))
        }
    }
    loop {
        let mut input = String::new();
        while stdin().read_line(&mut input).is_ok() {
            let i = std::mem::take(&mut input);
            if !tx.is_closed() {
                tx.send(i).expect("to not be closed");
            } else {
                break;
            }
        }
    }
}
#[instrument(skip(s, rx))]
async fn console_process(
    s: ShutdownManager<&'static str>,
    mut rx: UnboundedReceiver<String>,
    _: LurkChan,
    _: impl CacheHttp,
) {
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

                match Command::try_parse_from(msg).map(|e| e.command.clone()) {
                    Ok(Commands::Quit) => {
                        let _ = s.trigger_shutdown("Console request");
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
}
