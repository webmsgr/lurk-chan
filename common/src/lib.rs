use std::str::FromStr;

use serde::{Deserialize, Serialize};
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unknown location: {0}")]
    UnknownLocation(String),
}

/// A Report.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Report {
    /// The id of the reporter
    #[serde(alias = "Reporter UserID")]
    pub reporter_id: String,
    /// the name of the reporter.
    #[serde(alias = "Reporter Nickname")]
    pub reporter_name: String,
    /// the id of the reported
    #[serde(alias = "Reported UserID")]
    pub reported_id: String,
    /// the name of the reported
    #[serde(alias = "Reported Nickname")]
    pub reported_name: String,
    /// why was the reported, reported?
    #[serde(alias = "Reason")]
    pub report_reason: String,
    /// The status of the report
    #[serde(default)]
    pub report_status: ReportStatus,
    /// The server where the report came in
    #[serde(alias = "Server Name")]
    pub server: String,
    /// When the report came in
    #[serde(alias = "UTC Timestamp")]
    pub time: String,
    /// Who claimed it?
    #[serde(default)]
    pub claimant: Option<u64>,
    #[serde(default)]
    pub location: Location,
}
/// Various status of reports.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ReportStatus {
    #[default]
    Open,
    Claimed,
    Closed,
    Expired,
}

impl ReportStatus {
    /// convert a ReportStatus to a string identifying (for the database)
    pub fn to_db(&self) -> String {
        match self {
            Self::Open => "open",
            Self::Claimed => "claimed",
            Self::Closed => "closed",
            Self::Expired => "expired",
        }
        .to_string()
    }
    /// convert a database string to a ReportStatus
    pub fn from_db(item: &str) -> Option<Self> {
        match item {
            "open" => Some(Self::Open),
            "claimed" => Some(Self::Claimed),
            "closed" => Some(Self::Closed),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

/*target_id text not null,
target_username text not null,
offense text not null,
action text not null,
server text not null,
report int,*/

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Location {
    #[default]
    SL,
    Discord,
}

impl ToString for Location {
    fn to_string(&self) -> String {
        match self {
            Location::SL => "sl".to_string(),
            Location::Discord => "discord".to_string(),
        }
    }
}

impl FromStr for Location {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sl" => Ok(Self::SL),
            "discord" => Ok(Self::Discord),
            _ => Err(Error::UnknownLocation(s.to_string())),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub target_id: String,
    pub target_username: String,
    pub offense: String,
    pub action: String,
    pub server: Location,
    pub claimant: u64,
    pub report: Option<u32>,
}

#[cfg(test)]
mod tests {
    use crate::ReportStatus;

    #[test]
    fn test_report_status_from_db() {
        assert_eq!(Some(ReportStatus::Open), ReportStatus::from_db("open"));
        assert_eq!(Some(ReportStatus::Closed), ReportStatus::from_db("closed"));
        assert_eq!(
            Some(ReportStatus::Expired),
            ReportStatus::from_db("expired")
        );
        assert_eq!(
            Some(ReportStatus::Claimed),
            ReportStatus::from_db("claimed")
        );
        assert_eq!(None, ReportStatus::from_db("piss"));
    }
    #[test]
    fn test_report_status_to_db() {
        assert_eq!(&ReportStatus::Open.to_db(), "open");
        assert_eq!(&ReportStatus::Closed.to_db(), "closed");
        assert_eq!(&ReportStatus::Expired.to_db(), "expired");
        assert_eq!(&ReportStatus::Claimed.to_db(), "claimed");
    }
}
