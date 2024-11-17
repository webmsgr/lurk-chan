use poise::{serenity_prelude as serenity, Modal};

async fn execute_modal_generic<
    M: Modal,
    F: std::future::Future<Output = Result<(), serenity::Error>>,
>(
    ctx: &serenity::Context,
    create_interaction_response: impl FnOnce(serenity::CreateInteractionResponse) -> F,
    modal_custom_id: String,
    defaults: Option<M>,
    timeout: Option<std::time::Duration>,
) -> Result<Option<M>, serenity::Error> {
    // Send modal
    create_interaction_response(M::create(defaults, modal_custom_id.clone())).await?;

    // Wait for user to submit
    let response = serenity::collector::ModalInteractionCollector::new(&ctx.shard)
        .filter(move |d| d.data.custom_id == modal_custom_id)
        .timeout(timeout.unwrap_or(std::time::Duration::from_secs(3600)))
        .await;
    let response = match response {
        Some(x) => x,
        None => return Ok(None),
    };

    // Send acknowledgement so that the pop-up is closed
    response
        .create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
        .await?;

    Ok(Some(
        M::parse(response.data.clone()).map_err(serenity::Error::Other)?,
    ))
}

/// Convenience function for showing the modal and waiting for a response.
///
/// If the user doesn't submit before the timeout expires, `None` is returned.
///
/// Note: a modal must be the first response to a command. You cannot send any messages before,
/// or the modal will fail.
///
/// This function:
/// 1. sends the modal via [`Modal::create()`]
/// 2. waits for the user to submit via [`serenity::ModalInteractionCollector`]
/// 3. acknowledges the submitted data so that Discord closes the pop-up for the user
/// 4. parses the submitted data via [`Modal::parse()`], wrapping errors in [`serenity::Error::Other`]
///
/// If you need more specialized behavior, you can copy paste the implementation of this function
/// and adjust to your needs. The code of this function is just a starting point.
pub async fn execute_modal<U: Send + Sync, E, M: Modal>(
    ctx: poise::ApplicationContext<'_, U, E>,
    defaults: Option<M>,
    timeout: Option<std::time::Duration>,
) -> Result<Option<M>, serenity::Error> {
    let interaction = ctx.interaction;
    let response = execute_modal_generic(
        ctx.serenity_context(),
        |resp| interaction.create_response(ctx, resp),
        interaction.id.to_string(),
        defaults,
        timeout,
    )
    .await?;
    ctx.has_sent_initial_response
        .store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(response)
}

/// Convenience function for showing the modal on a message interaction and waiting for a response.
///
/// If the user doesn't submit before the timeout expires, `None` is returned.
///
/// This function:
/// 1. sends the modal via [`Modal::create()`] as a mci interaction response
/// 2. waits for the user to submit via [`serenity::ModalInteractionCollector`]
/// 3. acknowledges the submitted data so that Discord closes the pop-up for the user
/// 4. parses the submitted data via [`Modal::parse()`], wrapping errors in [`serenity::Error::Other`]
///
/// If you need more specialized behavior, you can copy paste the implementation of this function
/// and adjust to your needs. The code of this function is just a starting point.
pub async fn execute_modal_on_component_interaction<M: Modal>(
    ctx: &serenity::Context,
    interaction: serenity::ComponentInteraction,
    defaults: Option<M>,
    timeout: Option<std::time::Duration>,
) -> Result<Option<M>, serenity::Error> {
    execute_modal_generic(
        ctx,
        |resp| interaction.create_response(ctx, resp),
        interaction.id.to_string(),
        defaults,
        timeout,
    )
    .await
}
