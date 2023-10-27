mod audit;
mod audit_discord;
mod import;
mod judgement;
mod past;
mod ping;
mod report;
use anyhow::anyhow;
use serenity::all::{CommandInteraction, CreateInteractionResponse, CreateInteractionResponseMessage, Member, UserId};
use serenity::builder::CreateCommand;
use serenity::model::{application, Permissions};
use serenity::prelude::*;
use tracing::{error, info, instrument};

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
        _ => Err(anyhow!("Unknown command! {}", interact.data.name)),
    } {
        error!("Error running command {}: {:?}", interact.data.name, e)
    }
}

fn does_fit(user: &Member, perm: PermLevel, ctx: &Context) -> bool {
    match perm {
        PermLevel::Anyone => true,
        PermLevel::Staff => {
            user.roles
                .iter()
                .any(|role| {
                    role.to_role_cached(&ctx).is_some_and(|e| {
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
        "ping" => Anyone,
        "audit" | "discord" | "Audit Message" | "Audit User" | "past" | "report" => Staff,
        "import" => Dev,
        "when" => God,
        _ => Invalid
    }
}

fn do_perms(c: CreateCommand) -> CreateCommand {
    c.dm_permission(false)
}

macro_rules! command {
    ($ctx:expr, $name:ident) => {
        let (command, name) = $name::register();
        let perms = perm_level(name);
        info!("Registering command: {} with perms: {:?}", stringify!($name), perms);
        assert_ne!(perms, crate::commands::PermLevel::Invalid);
        application::Command::create_global_command($ctx, do_perms(command))
            .await
            .expect(&format!(
                "Failed to create global command {}",
                stringify!($name)
            ))
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
        application::Command::create_global_command($ctx, do_perms(command))
            .await
            .expect(&format!(
                "Failed to create global command {}::{}",
                stringify!($name),
                stringify!($sub)
            ))
    };
}
#[instrument(skip(ctx))]
pub async fn register_commands(ctx: &impl CacheHttp) {
    let commands = application::Command::get_global_commands(ctx.http())
        .await
        .unwrap();
    for command in commands {
        info!("Unregistering command {}", command.name);
        application::Command::delete_global_command(ctx.http(), command.id)
            .await
            .unwrap();
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
    info!("All commands registered!")
}
