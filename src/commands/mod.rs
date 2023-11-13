mod audit;
mod audit_discord;
mod import;
mod judgement;
mod past;
mod ping;
mod report;
mod report_message_to_admins;
mod search;

use std::env::var;
use anyhow::anyhow;
use once_cell::sync::Lazy;

use serenity::all::{CommandInteraction, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId, Member, UserId};
use serenity::builder::{CreateCommand, CreateInteractionResponseFollowup};
use serenity::model::{application, Permissions};
use serenity::prelude::*;
use tracing::{error, info, instrument};

static DEV_GUILD: Lazy<GuildId> = Lazy::new(|| var("DEV_GUILD").ok().map(|i| GuildId::new(i.parse().expect("Failed to load dev_guild"))).unwrap());

pub async fn run_command(ctx: &Context, interact: &CommandInteraction) {
    let cmd = interact.data.name.as_str();
    let req_perm_level = perm_level(cmd);
    if !does_fit(interact.member.as_ref().expect("a member"), req_perm_level, ctx) {
        let _ = interact
            .create_response(
                &ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("nah buddy, permission denied, you need to be a '{}' to run this command", perm_level(cmd).as_ref()))
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }
    if let Err(e) = match cmd {
        "ping" => ping::run(ctx, interact).await,
        "audit" => audit::run(ctx, interact).await,
        "discord" | "Audit Message" | "Audit User" => audit_discord::run(ctx, interact).await,
        "past" => past::run(ctx, interact).await,
        "report" => report::run(ctx, interact).await,
        "import" => import::run(ctx, interact).await,
        "judgement" => judgement::run(ctx, interact).await,
        "Report Message" => report_message_to_admins::run(ctx, interact).await,
        "search" => search::run(ctx, interact).await,
        _ => Err(anyhow!("Unknown command! {}", interact.data.name)),
    } {
        error!("Error running command {}: {:?}", interact.data.name, e);
        let _ = interact.create_followup(&ctx, CreateInteractionResponseFollowup::new()
            .content("Hey, that command returned an error. Might be your fault, might be mine. Who knows.").ephemeral(true)
        ).await;
    }
}

fn does_fit(user: &Member, perm: PermLevel, ctx: &Context) -> bool {
    match perm {
        PermLevel::Anyone => true,
        PermLevel::Staff => {
            user.roles
                .iter()
                .any(|role| {
                    role.to_role_cached(ctx).is_some_and(|e| {
                        e.permissions.contains(Permissions::ADMINISTRATOR)
                            || e.name.contains("Admin")
                            || e.name.contains("Mod")
                    })
                })
        },
        PermLevel::Dev => user.user.id == OWNER,
        PermLevel::God | PermLevel::Nobody | PermLevel::Invalid => false
    }
}


const OWNER: UserId = UserId::new(171629704959229952);
use strum::{EnumVariantNames, EnumString, AsRefStr};
#[derive(Debug, PartialEq, Eq, EnumVariantNames, EnumString, AsRefStr)]
enum PermLevel {
    Anyone,
    Staff,
    Dev,
    God,
    Nobody,
    Invalid
}

fn perm_level(command: &str) -> PermLevel {
    use crate::commands::PermLevel::*;
    match command {
        "ping" | "Report Message" => Anyone,
        "audit" | "discord" | "Audit Message" | "Audit User" | "past" | "report" | "search" => Staff,
        "import" => Dev,
        "when" => God,
        _ => Invalid
    }
}

fn do_perms(c: CreateCommand) -> CreateCommand {
    c.dm_permission(false)
}


macro_rules! register_command {
    ($ctx:expr, $cmd:expr) => {
        if cfg!(debug_assertions) {
            DEV_GUILD.create_command($ctx, $cmd).await
        } else {
            application::Command::create_global_command($ctx, $cmd)
            .await
        }
    };
}

macro_rules! command {
    ($ctx:expr, $name:ident) => {
        let (command, name) = $name::register();
        let perms = perm_level(name);
        info!("Registering command: {} with perms: {:?}", stringify!($name), perms);
        assert_ne!(perms, crate::commands::PermLevel::Invalid);
        register_command!($ctx, do_perms(command)).expect(&format!(
                "Failed to create global command {}",
                stringify!($name)
            ));
    };
    ($ctx:expr, $name:ident, $sub:ident) => {
        let (command, name) = $name::$sub::register();
        let perms = perm_level(name);
        info!(
            "Registering command: {}::{} with perms {:?}",
            stringify!($name),
            stringify!($sub),
            perms
        );
        assert_ne!(perms, crate::commands::PermLevel::Invalid);
        register_command!($ctx, do_perms(command))
            .expect(&format!(
                "Failed to create global command {}::{}",
                stringify!($name),
                stringify!($sub)
            ))
    };
}
#[instrument(skip(ctx))]
pub async fn register_commands(ctx: &impl CacheHttp) {
    if cfg!(debug_assertions) {
        info!("Commands will be registered in debug server only");
    } else {
        info!("Commands will be registered in all guilds");
    }
    let commands = if cfg!(debug_assertions) {
        DEV_GUILD.get_commands(ctx.http()).await.unwrap()
    } else {
        application::Command::get_global_commands(ctx.http())
            .await
            .unwrap()
    };
    for command in commands {
        info!("Unregistering command {}", command.name);
        if cfg!(debug_assertions) {
            DEV_GUILD.delete_command(ctx.http(), command.id).await.unwrap();
        } else {
            application::Command::delete_global_command(ctx.http(), command.id)
                .await
                .unwrap();
        }
    }
    command!(ctx, ping);
    command!(ctx, audit);
    command!(ctx, audit_discord, user);
    command!(ctx, audit_discord, slash);
    command!(ctx, audit_discord, message);
    command!(ctx, past);
    command!(ctx, report);
    command!(ctx, import);
    command!(ctx, judgement);
    command!(ctx, report_message_to_admins);
    command!(ctx, search);
    info!("All commands registered!")
}
