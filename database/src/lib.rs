use std::str::FromStr;

use common::{Report, ReportStatus, Action, Location};
use sqlx::{SqlitePool, sqlite::{SqlitePoolOptions, SqliteConnectOptions}, migrate};


#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Database migration error: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),
    #[error("Invalid report status: {0}")]
    InvalidReportStatus(String),
    #[error("Failed to parse number: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("{0}")]
    CommonError(#[from] common::Error)
}

pub struct Database {
    pool: SqlitePool
}

impl Database {
    pub async fn new() -> Result<Self, Error> {
        let options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename("lurk_chan.db");
        let pool = SqlitePoolOptions::new()
            .connect_with(options).await?;
        migrate!().run(&pool).await?;
        return Ok(Self {
            pool
        })
    }
    pub async fn vacuum(&self) -> Result<(), Error> {
        sqlx::query("vacuum;").execute(&self.pool).await?;
        Ok(())
    }
    pub async fn get_report_from_id(&self, report_id: u32) -> Result<Option<Report>, Error> {
        let report_id = report_id as i64;
        let res: Option<DBReport> = sqlx::query_as!(DBReport, "select * from Reports where id = ?", report_id)
            .fetch_optional(&self.pool).await?;
        match res {
            Some(i) => Ok(Some(i.into_report()?)),
            None => Ok(None)
        }
    }
    pub async fn add_report(&self, report: Report) -> Result<u32, Error> {
        let r = DBReport::from_report(report);
        let res = 
            sqlx::query!(
                "insert into Reports(reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status, server, time, claimant, audit) values (?,?,?,?,?,?,?,?,?,?)", 
                r.reporter_id,
                r.reporter_name,
                r.reported_id,
                r.reported_name,
                r.report_reason,
                r.report_status,
                r.server,
                r.time,
                r.claimant,
                r.audit
            ).execute(&self.pool).await?;
        Ok(res.last_insert_rowid() as u32)
    }
    pub async fn get_action_from_id(&self, id: u32)  -> Result<Option<Action>, Error>   {
        let action_id = id as i64;
        let res: Option<DBAction> = sqlx::query_as!(DBAction, "select * from Actions where id = ?", action_id)
            .fetch_optional(&self.pool).await?;
        match res {
            Some(i) => Ok(Some(i.try_into()?)),
            None => Ok(None)
        }
    }
}

/*id integer primary key,
reporter_id text not null,
reporter_name text not null,
reported_id text not null,
reported_name text not null,
report_reason text not null,
report_status text not null,
server text not null,
time text not null,
claimant text,
audit text*/
struct DBReport {
    /// always Some() when coming from DB, None when coming from From<Report>
    id: Option<i64>,
    reporter_id: String,
    reporter_name: String,
    reported_id: String,
    reported_name: String,
    report_reason: String,
    report_status: String,
    server: String,
    time: String,
    claimant: Option<String>,
    audit: Option<String>
}

impl DBReport {
    fn into_report(self) -> Result<Report, Error> {
        Ok(Report {
            reporter_id: self.reporter_id,
            reporter_name: self.reporter_name,
            reported_id: self.reported_id,
            reported_name: self.reported_name,
            report_reason: self.report_reason,
            report_status: ReportStatus::from_db(&self.report_status).ok_or_else(|| Error::InvalidReportStatus(self.report_status))?,
            server: self.server,
            time: self.time,
            claimant: match self.claimant {
                Some(i) => Some(i.parse()?),
                None => None
            },
            audit: match self.audit {
                Some(i) => Some(i.parse()?),
                None => None
            }
        })
    }
    fn from_report(r: Report) -> Self {
        Self {
            id: None,
            reporter_id: r.reporter_id,
            reporter_name: r.reporter_name,
            reported_id: r.reported_id,
            reported_name: r.reported_name,
            report_reason: r.report_reason,
            report_status: r.report_status.to_db(),
            server: r.server,
            time: r.time,
            claimant: r.claimant.map(|i| i.to_string()),
            audit: r.audit.map(|i| i.to_string())
        }
    }
}

struct DBAction {
    pub id: Option<i64>,
    pub target_id: String,
    pub target_username: String,
    pub offense: String,
    pub action: String,
    pub server: String,
    pub claimant: String,
    pub report: Option<i64>,
}

impl From<Action> for DBAction {
    fn from(value: Action) -> Self {
        Self {
            id: None,
            report: value.report.map(|i| i as i64),
            server: value.server.to_string(),
            target_id: value.target_id,
            target_username: value.target_username,
            offense: value.offense,
            action: value.action,
            claimant: value.claimant.to_string()
        }
    }
}
impl TryInto<Action> for DBAction {
    type Error = Error;
    fn try_into(self) -> Result<Action, Self::Error> {
        Ok(Action {
            report: self.report.map(|i| i as u32),
            target_id: self.target_id,
            target_username: self.target_username,
            offense: self.offense,
            action: self.action,
            claimant: self.claimant.parse()?,
            server: Location::from_str(&self.server)?
        })
    }
}