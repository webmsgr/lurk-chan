mod past;
mod audit;
mod audit_discord;
mod ping;
mod report;
mod import;

use serenity::all::{CommandInteraction, CommandType, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::{application, Permissions};
use serenity::prelude::*;
use tracing::{error, info, instrument};

pub async fn run_command(ctx: &Context, interact: &CommandInteraction) {
    if !interact.member.as_ref().expect("Not in a fucking DM").roles.iter().any(|role| {
        role.to_role_cached(&ctx).is_some_and(|e| {
            e.permissions.contains(Permissions::ADMINISTRATOR) || e.name.contains("Admin") || e.name.contains("Mod")
        })
    }) {
        let _ = interact.create_response(&ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("nah oomfie you dont have the roles").ephemeral(true))).await;
        return;
    }
    if let Err(e) = match interact.data.name.as_str() {
        "ping" => ping::run(ctx, interact).await,
        "audit" => audit::run(ctx, interact).await,
        "discord" | "discord_audit" => audit_discord::run(ctx, interact).await,
        "past" => past::run(ctx, interact).await,
        "report" => report::run(ctx, interact).await,
        "import" => import::run(ctx, interact).await,
         _ => Err("Unknown command!".into())
    } {
        error!("Error running command {}: {}", interact.data.name, e)
    }
}



fn do_perms(c: CreateCommand) -> CreateCommand {
    c.dm_permission(false)
}

macro_rules! command {
    ($ctx:expr, $name:ident) => {
        info!("Registering command: {}", stringify!($name));
        application::Command::create_global_command($ctx, do_perms($name::register())).await.expect(&format!("Failed to create global command {}", stringify!($name)))
    };
    ($ctx:expr, $name:ident, $sub:ident) => {
        info!("Registering command: {}::{}", stringify!($name), stringify!($sub));
        application::Command::create_global_command($ctx, do_perms($name::$sub::register())).await.expect(&format!("Failed to create global command {}::{}", stringify!($name), stringify!($sub)))
    }
}
#[instrument(skip(ctx))]
pub async fn register_commands(ctx: &impl CacheHttp) {
    let commands = application::Command::get_global_commands(ctx.http()).await.unwrap();
    for command in commands {
        info!("Unregistering command {}", command.name);
        application::Command::delete_global_command(ctx.http(), command.id).await.unwrap();
    }
    command!(ctx, ping);
    command!(ctx, audit);
    command!(ctx, audit_discord, user);
    command!(ctx, audit_discord, slash);
    command!(ctx, past);
    command!(ctx, report);
    command!(ctx, import);
    info!("All commands registered!")
}