use std::time::Duration;
mod audit;
mod move_thing;
mod past;
mod ping;
mod report;
mod report_to_admin;

pub fn commands() -> Vec<poise::Command<crate::LurkChan, anyhow::Error>> {
    vec![
        ping::ping(),
        report::report(),
        audit::audit(),
        past::past(),
        report_to_admin::report_to_admins(),
        move_thing::move_command(),
    ]
    .into_iter()
    .map(|mut i| {
        i.guild_only = true;
        i.subcommand_required = true;
        //i.default_member_permissions = Permissions::MUTE_MEMBERS;
        /*i.checks
        .push(|ctx: crate::Context| Box::pin(async move { check_perms(ctx).await }));*/
        i.cooldown_config.write().unwrap().user = Some(Duration::from_secs(5));
        i
    })
    .collect()
}

/*async fn check_perms(ctx: crate::Context<'_>) -> anyhow::Result<bool> {
    if ctx
        .author_member()
        .await
        .context("nah")?
        .roles
        .iter()
        .any(|role| {
            role.to_role_cached(ctx).is_some_and(|e| {
                e.permissions.contains(Permissions::ADMINISTRATOR)
                    || e.name.contains("Admin")
                    || e.name.contains("Mod")
            })
        }) {
            Ok(true)
        } else {
            ctx.send(
                CreateReply::default()
                    .content("You don't have permission to do that!")
                    .ephemeral(true),
            )
            .await?;
            Ok(false)
        }
}*/
