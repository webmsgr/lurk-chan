use crate::report::Report;
use serenity::builder::*;
//use serenity::model::prelude::component::*;
use crate::audit::Location;
use serenity::model::prelude::*;

pub fn audit_log_modal(
    id: Option<i64>,
    specifier: char,
    report: Option<Report>,
    default_location: Location,
    prefill_punishment: Option<String>,
) -> CreateInteractionResponse {
    let title = if id.is_some() {
        "Close report"
    } else {
        "Audit Log"
    };
    CreateInteractionResponse::Modal(
        CreateModal::new(format!("{}{}", specifier, id.unwrap_or(-1)), title).components(vec![
            CreateActionRow::InputText(
                CreateInputText::new(
                    InputTextStyle::Short,
                    "Location ('SL' or 'Discord')",
                    "location",
                )
                .value(match default_location {
                    Location::Discord => "Discord",
                    Location::SL => "SL",
                }),
            ),
            CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Short, "ID", "id").value(
                    report
                        .as_ref()
                        .and_then(|r| Some(r.reported_id.as_str()))
                        .unwrap_or(""),
                ),
            ),
            CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Short, "Name", "name").value(
                    report
                        .as_ref()
                        .and_then(|r| Some(r.reported_name.as_str()))
                        .unwrap_or(""),
                ),
            ),
            CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Short, "Offense", "offense").value(
                    report
                        .as_ref()
                        .and_then(|r| Some(r.report_reason.as_str()))
                        .unwrap_or(""),
                ),
            ),
            CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Short, "Punishment", "punishment").value(
                    prefill_punishment
                        .unwrap_or_else(|| "".to_string())
                        .as_str(),
                ),
            ),
        ]),
    )
}
