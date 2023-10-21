use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serenity::model::id::ChannelId;
use std::env::var;
use serenity::builder::{CreateEmbed, CreateEmbedAuthor};
use serenity::client::Context;
use serenity::model::prelude::*;
use std::result::Result;
use serenity::builder::CreateEmbedFooter;
use serenity::model::Color;

pub static SL_AUDIT: Lazy<ChannelId> = Lazy::new(|| var("SL_AUDIT").unwrap().parse().unwrap());
pub static DISC_AUDIT: Lazy<ChannelId> = Lazy::new(|| var("DISC_AUDIT").unwrap().parse().unwrap());

#[derive(Debug, Default, Serialize, Deserialize, sqlx::Type, Clone)]
#[sqlx(rename_all = "lowercase")]
pub enum Location {
    #[default]
    SL,
    Discord,
}
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AuditModelResult {
    pub location: Location,
    pub punishment: String,
    pub offense: String,
    pub name: String,
    pub id: String,
}

impl AuditModelResult {
    pub fn to_action(self, report_id: Option<i64>, user: UserId) -> Action {
        Action {
            target_username: self.name,
            target_id: self.id,
            offense: self.offense,
            action: self.punishment,
            server: self.location,
            claimant: user.get().to_string(),
            report: report_id
        }
    }
}


/*target_id text not null,
target_username text not null,
offense text not null,
action text not null,
server text not null,
report int,*/
#[derive(Clone, Debug)]
pub struct Action {
    pub target_id: String,
    pub target_username: String,
    pub offense: String,
    pub action: String,
    pub server: Location,
    pub claimant: String,
    pub report: Option<i64>
}
impl Action {
    pub async fn create_embed(self, ctx: &Context) -> Result<CreateEmbed, Box<dyn std::error::Error + Send + Sync>> {
        let c: UserId = self.claimant.parse()?;
        let g = SL_AUDIT.to_channel(ctx).await?.guild().ok_or_else(|| "fuck")?.guild(ctx).unwrap().id;
        let u = c.to_user(ctx).await?;
        let nick = u.nick_in(ctx, g).await.unwrap_or_else(|| u.global_name.as_ref().unwrap_or_else(|| &u.name).clone());
        //let ch = SL_AUDIT.to_channel(ctx).await.unwrap().g;
        Ok(CreateEmbed::default()
            .title("Audit Log")
            .color(Color::PURPLE/*from_rgb(249,19,109)*/)
            .author(CreateEmbedAuthor::new(nick).icon_url(u.face()))
            .field("ID", self.target_id, false)
            .field("Username", self.target_username, false)
            .field("Offense", self.offense, false)
            .field("Action", self.action, false)
            .footer(CreateEmbedFooter::new({
                if let Some(r) = self.report {
                    format!("/report {}",r)
                } else {
                    "No report".to_string()
                }
            })))
    }
}
