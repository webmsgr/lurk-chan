use serde::{Serialize, Deserialize};

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
    pub claimant: Option<u64>,
    #[serde(default)]
    pub audit: Option<u64>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ReportStatus {
    #[default]
    Open,
    Claimed,
    Closed,
    Expired,
}

impl ReportStatus {
    pub fn to_db(&self) -> String {
        match self {
            Self::Open => "open",
            Self::Claimed => "claimed",
            Self::Closed => "closed",
            Self::Expired => "expired",
        }.to_string()
    }
    pub fn from_db(item: &str) -> Option<Self> {
        match item {
            "open" => Some(Self::Open),
            "claimed" => Some(Self::Claimed),
            "closed" => Some(Self::Closed),
            "expired" => Some(Self::Expired),
            _ => None
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::ReportStatus;

    #[test]
    fn test_report_status_from_db() {
        assert_eq!(Some(ReportStatus::Open), ReportStatus::from_db("open"));
        assert_eq!(Some(ReportStatus::Closed), ReportStatus::from_db("closed"));
        assert_eq!(Some(ReportStatus::Expired), ReportStatus::from_db("expired"));
        assert_eq!(Some(ReportStatus::Claimed), ReportStatus::from_db("claimed"));
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
