use anyhow::Context;
use poise::serenity_prelude::Permissions;

mod ping;
mod report;
mod audit;
pub fn commands() -> Vec<poise::Command<crate::LurkChan, anyhow::Error>> {
    vec![ping::ping(), report::report(), audit::audit()].into_iter().map(|mut i| {
        i.guild_only = true;
        i.subcommand_required = true;
        i.checks.push(|ctx: crate::Context| Box::pin(async move {
            check_perms(ctx).await
        }));
        i
    }).collect()
}

async fn check_perms(ctx: crate::Context<'_>) -> anyhow::Result<bool> {
    Ok(ctx.author_member().await.context("nah")?.roles.iter().any(|role| {
        role.to_role_cached(ctx).is_some_and(|e| {
            e.permissions.contains(Permissions::ADMINISTRATOR)
                || e.name.contains("Admin")
                || e.name.contains("Mod")
        })
    }))
}