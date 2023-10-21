use serde::{Deserialize, Serialize};
use serenity::all::CreateEmbedFooter;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor};
use serenity::model::prelude::*;
//use serenity::mde::Color;
use crate::audit::SL_AUDIT;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Report {
    #[serde(alias = "Reporter UserID")]
    pub reporter_id: String,
    #[serde(alias = "Reporter Nickname")]
    pub reporter_name: String,
    #[serde(alias = "Reported UserID")]
    pub reported_id: String,
    #[serde(alias = "Reported Nickname")]
    pub reported_name: String,
    #[serde(alias = "Reason")]
    pub report_reason: String,
    #[serde(default)]
    pub report_status: ReportStatus,
    #[serde(alias = "Server Name")]
    pub server: String,
    #[serde(alias = "UTC Timestamp")]
    pub time: String,
    #[serde(default)]
    pub claimant: Option<String>,
    #[serde(default)]
    pub audit: Option<String>,
}

impl Report {
    pub fn create_embed(self) -> CreateEmbed {
        let rs = self.report_status_string();
        CreateEmbed::default().title("Report")
            .description("A new report just came in!")
            .field("Reporter ID", self.reporter_id, true)
            .field("Reporter Nickname", self.reporter_name, true)
            .field("", "", false)
            .field("Reported ID", self.reported_id.clone(), true)
            .field("Reported Nickname", self.reported_name, true)
            .field("", "", false)
            .field("Report Reason", self.report_reason, true)
            .field(
                "Report Status",
                rs,
                true,
            )
            .color(match self.report_status {
                ReportStatus::Open => Color::from_rgb(0, 255, 0),
                ReportStatus::Claimed => Color::from_rgb(255, 255, 0),
                ReportStatus::Closed => Color::from_rgb(255, 0, 0),
            })
            .author(CreateEmbedAuthor::new(self.server).icon_url("https://i.imgur.com/4jVFfFM.webp"))
            .footer(CreateEmbedFooter::new(format!("/past {}", self.reported_id)))
            .timestamp(self.time.parse::<Timestamp>().expect("SL gives a good time"))
    }
    pub fn report_status_string(&self) -> String{
        match self.report_status.clone() {
            ReportStatus::Open => "Open".to_string(),
            ReportStatus::Claimed => match &self.claimant {
                Some(id) => format!("Claimed by <@!{}>", id),
                None => "Claimed by ???".to_string(),
            },
            ReportStatus::Closed => {
                let mut s = "Closed by ".to_string();
                if let Some(claim) = &self.claimant {
                    s.push_str(&format!("<@!{}>", claim));
                } else {
                    s.push_str("???")
                }
                if let Some(audit) = &self.audit {
                    let msg =
                        MessageId::new(audit.parse().expect(
                            "Invalid sqlite data, audit needs to be a message id",
                        ));
                    s.push_str(&format!(" (See {})", msg.link(*SL_AUDIT, None)));
                }
                s
            }
        }
    }
    pub fn components(&self, id: i64) -> Vec<CreateActionRow> {
        let i: Option<CreateActionRow> = match self.report_status {
            ReportStatus::Open => {
                Some(CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("claim_{}", id)).label("Claim").style(ButtonStyle::Primary),
                    CreateButton::new(format!("close_{}", id)).label("Close").style(ButtonStyle::Primary),
                    CreateButton::new(format!("forceclose_{}", id)).label("Close without action").style(ButtonStyle::Danger),
                ]))
            },
            ReportStatus::Claimed => {
                Some(CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("close_{}", id)).label("Close").style(ButtonStyle::Primary),
                    CreateButton::new(format!("forceclose_{}", id)).label("Close without action").style(ButtonStyle::Danger),
                ]))
            }

            ReportStatus::Closed => None
        };
        if let Some(a) = i {
            vec![a]
        } else {
            vec![]
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, sqlx::Type, Clone)]
#[sqlx(rename_all = "lowercase")]
pub enum ReportStatus {
    #[default]
    Open,
    Claimed,
    Closed,
}
