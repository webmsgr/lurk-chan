use std::borrow::Cow;

use common::{Action, Location, Report, ReportStatus};
use database::Database;
use poise::serenity_prelude::{
    ButtonStyle, CacheHttp, ChannelId, Color, CreateActionRow, CreateButton, CreateEmbed,
    CreateEmbedAuthor, CreateEmbedFooter, EditMessage, MessageId, Timestamp, UserId,
};
use serde::{de::DeserializeOwned, Serialize};
/// stupid idiot function to convert serializable to serializable.
/// Useful for hashmap -> object conversions (how its used in LC)
pub fn transmute_json<I: Serialize, D: DeserializeOwned>(from: I) -> Result<D, serde_json::Error> {
    serde_json::to_value(from).and_then(serde_json::from_value::<D>)
}

pub fn do_sl_subs(input: &str) -> Cow<str> {
    if input.contains("[QM]") || input.contains("[AMP]") {
        let mut s = input.to_string();
        s = s
            .replace(" [QM] ", "?")
            .replace(" [QM]", "?")
            .replace("[QM] ", "?");
        s = s
            .replace(" [AMP] ", "&")
            .replace(" [AMP]", "&")
            .replace("[AMP] ", "&");
        Cow::Owned(s)
    } else {
        Cow::Borrowed(input)
    }
}

mod modal_bullshit;
pub use modal_bullshit::*;
pub async fn create_things_from_report(
    r: Report,
    rid: u32,
    db: &Database,
) -> anyhow::Result<(CreateEmbed, Vec<CreateActionRow>)> {
    tokio::try_join!(
        create_report_embed(&r, rid, db),
        create_report_action_row(&r, rid, db)
    )
}

pub async fn create_report_embed(
    r: &Report,
    rid: u32,
    db: &Database,
) -> anyhow::Result<CreateEmbed> {
    let report_count = db.get_report_count(&r.reported_id).await?;
    let rs = {
        match r.report_status.clone() {
            ReportStatus::Open => "Open".to_string(),
            ReportStatus::Expired => "Expired".to_string(),
            ReportStatus::Claimed => match &r.claimant {
                Some(id) => format!("Claimed by <@!{}>", id),
                None => "Claimed by ???".to_string(),
            },
            ReportStatus::Closed => {
                let mut s = "Closed by ".to_string();
                if let Some(claim) = &r.claimant {
                    s.push_str(&format!("<@!{}>", claim));
                } else {
                    s.push_str("???")
                }
                if let Some((chan, audit)) = db.get_action_message_from_report_id(rid).await? {
                    let chan = ChannelId::new(chan);
                    let msg = MessageId::new(audit);
                    s.push_str(&format!(" (See {})", msg.link(chan, None)));
                }
                s
            }
        }
    };
    Ok(CreateEmbed::default()
            .title(format!("Report #{}", rid))
            .description("A new report just came in!")
            .field("Reporter ID", do_sl_subs(&r.reporter_id).into_owned(), true)
            .field("Reporter Nickname", do_sl_subs(&r.reporter_name).into_owned(), true)
            .field("", "", false)
            .field("Reported ID", do_sl_subs(&r.reported_id).into_owned(), true)
            .field("Reported Nickname", do_sl_subs(&r.reported_name).into_owned(), true)
            .field("", "", false)
            .field("Report Reason", do_sl_subs(&r.report_reason).into_owned(), true)
            .field("Report Status", rs, true)
            .color(match r.report_status {
                ReportStatus::Open => Color::from_rgb(0, 255, 0),
                ReportStatus::Claimed => Color::from_rgb(255, 255, 0),
                ReportStatus::Closed => Color::from_rgb(255, 0, 0),
                ReportStatus::Expired => Color::LIGHT_GREY,
            })
            .author(
                CreateEmbedAuthor::new(r.server.clone()).icon_url(match r.location {
                    Location::SL => "https://cdn2.steamgriddb.com/file/sgdb-cdn/icon/0b1a888bc5720fc6b2a1585f802f6964/32/256x256.png",
                    Location::Discord => "https://i.imgur.com/vkdFsjQ.png"
                }),
            )
            .footer(CreateEmbedFooter::new(format!(
                "`/past who:{}` (has been reported {} times)",
                r.reported_id, report_count
            )))
            .timestamp(
                r.time
                    .parse::<Timestamp>()
                    .expect("SL gives a good time"),
            )
        )
}

pub async fn create_report_action_row(
    r: &Report,
    id: u32,
    _db: &Database,
) -> anyhow::Result<Vec<CreateActionRow>> {
    let i: Option<CreateActionRow> = match r.report_status {
        ReportStatus::Open => Some(CreateActionRow::Buttons(vec![CreateButton::new(format!(
            "claim_{}",
            id
        ))
        .label("Claim")
        .style(ButtonStyle::Primary)])),
        ReportStatus::Expired => Some(CreateActionRow::Buttons(vec![CreateButton::new(format!(
            "claim_{}",
            id
        ))
        .label("Reopen and Claim")
        .style(ButtonStyle::Secondary)])),
        ReportStatus::Claimed => Some(CreateActionRow::Buttons(vec![
            CreateButton::new(format!("close_{}", id))
                .label("Close")
                .style(ButtonStyle::Primary),
            CreateButton::new(format!("forceclose_{}", id))
                .label("Close without action")
                .style(ButtonStyle::Danger),
        ])),

        ReportStatus::Closed => None,
    };
    Ok(if let Some(a) = i { vec![a] } else { vec![] })
}

pub async fn update_report_message(
    ctx: &impl CacheHttp,
    rid: u32,
    db: &Database,
) -> anyhow::Result<()> {
    let (embed, comp) = create_things_from_report(
        db.get_report_from_id(rid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Report not found!"))?,
        rid,
        db,
    )
    .await?;
    let (chan, mes) = db
        .get_report_message(rid)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Report message not found!"))?;
    let chan = ChannelId::new(chan);
    let msg = MessageId::new(mes);
    chan.edit_message(
        ctx,
        msg,
        EditMessage::default().embed(embed).components(comp),
    )
    .await?;
    Ok(())
}

pub async fn create_action_embed(
    action: &Action,
    ctx: &impl CacheHttp,
    id: u32,
    channel: ChannelId,
) -> anyhow::Result<CreateEmbed> {
    let c: UserId = UserId::new(action.claimant);
    let g = channel
        .to_channel(ctx)
        .await?
        .guild()
        .ok_or_else(|| anyhow::anyhow!("fuck"))?
        .guild(ctx.cache().expect("what"))
        .unwrap()
        .id;
    let u = c.to_user(ctx).await?;
    let nick = u
        .nick_in(ctx, g)
        .await
        .unwrap_or_else(|| u.global_name.as_ref().unwrap_or(&u.name).clone());
    //let ch = SL_AUDIT.to_channel(ctx).await.unwrap().g;
    Ok(CreateEmbed::default()
        .title(format!("Audit Log #{}", id))
        .color(Color::PURPLE /*from_rgb(249,19,109)*/)
        .author(CreateEmbedAuthor::new(nick).icon_url(u.face()))
        .field("ID", action.target_id.clone(), false)
        .field(
            "Username",
            do_sl_subs(&action.target_username).into_owned(),
            false,
        )
        .field("Offense", do_sl_subs(&action.offense).into_owned(), false)
        .field("Action", do_sl_subs(&action.action).into_owned(), false)
        .footer(CreateEmbedFooter::new({
            if let Some(r) = action.report {
                format!("/report report_id:{}", r)
            } else {
                "No report".to_string()
            }
        })))
}
pub fn create_action_components(id: u32) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![CreateButton::new(format!(
        "edit_{}",
        id
    ))
    .label("Edit")
    .style(ButtonStyle::Secondary)])]
}

pub async fn update_audit_message(
    ctx: &impl CacheHttp,
    id: u32,
    db: &Database,
) -> anyhow::Result<()> {
    let action = db
        .get_action_from_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Action not found!"))?;
    let (chan, mes) = db
        .get_action_message(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Action message not found!"))?;
    let chan = ChannelId::new(chan);
    let (embed, comp) = tokio::try_join!(create_action_embed(&action, ctx, id, chan), async {
        Ok(create_action_components(id))
    })?;

    let msg = MessageId::new(mes);
    chan.edit_message(
        ctx,
        msg,
        EditMessage::default().embed(embed).components(comp),
    )
    .await?;
    Ok(())
}
