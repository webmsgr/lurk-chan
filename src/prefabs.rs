use crate::{audit::Action, report::Report};
use serenity::builder::*;
//use serenity::model::prelude::component::*;
use crate::audit::Location;
use serenity::model::prelude::*;

#[derive(Debug, Default)]
pub struct AutofillAuditLog {
    pub location: Location,
    pub id: Option<String>,
    pub name: Option<String>,
    pub offense: Option<String>,
    pub punishment: Option<String>,
}

impl From<Report> for AutofillAuditLog {
    fn from(value: Report) -> Self {
        Self {
            location: Location::SL,
            id: Some(value.reported_id),
            name: Some(value.reported_name),
            offense: Some(value.report_reason),
            punishment: None,
        }
    }
}

impl From<Action> for AutofillAuditLog {
    fn from(value: Action) -> Self {
        Self {
            location: value.server,
            id: Some(value.target_id),
            name: Some(value.target_username),
            offense: Some(value.offense),
            punishment: Some(value.action),
        }
    }
}

pub fn audit_log_modal(
    id: Option<i64>,
    specifier: char,
    autofill: Option<AutofillAuditLog>,
) -> CreateInteractionResponse {
    let title = if id.is_some() {
        "Close report"
    } else {
        "Audit Log"
    };
    let autofill = autofill.unwrap_or_default();
    CreateInteractionResponse::Modal(
        CreateModal::new(format!("{}{}", specifier, id.unwrap_or(-1)), title).components(vec![
            CreateActionRow::InputText(
                CreateInputText::new(
                    InputTextStyle::Short,
                    "Location ('SL' or 'Discord')",
                    "location",
                )
                .value(match autofill.location {
                    Location::Discord => "Discord",
                    Location::SL => "SL",
                }),
            ),
            CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Short, "ID", "id")
                    .value(autofill.id.unwrap_or_default()),
            ),
            CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Short, "Name", "name")
                    .value(autofill.name.unwrap_or_default()),
            ),
            CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Short, "Offense", "offense")
                    .value(autofill.offense.unwrap_or_default()),
            ),
            CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Short, "Punishment", "punishment")
                    .value(autofill.punishment.unwrap_or_default()),
            ),
        ]),
    )
}
