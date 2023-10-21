use std::error::Error;
use std::io::{stdin, IsTerminal};
use std::thread;

use async_shutdown::ShutdownManager;
use serenity::model::id::GuildId;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{error, info, instrument};

#[instrument(skip(s))]
pub fn spawn_console(s: ShutdownManager<&'static str>) {
    let dead_token = s.trigger_shutdown_token("Console thread died.");
    let (tx, rx) = unbounded_channel();
    thread::spawn(move || {
        console_thread(tx);
        info!("Shuttdown triggered because read_console thread fucking died");
        drop(dead_token); //t
    });
    let s2 = s.clone();
    tokio::task::spawn(
        s.wrap_delay_shutdown(console_process(s2, rx))
            .expect("failed to create console task"),
    );
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
#[instrument(skip(s, rx))]
async fn console_process(s: ShutdownManager<&'static str>, mut rx: UnboundedReceiver<String>) {
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
                    }
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