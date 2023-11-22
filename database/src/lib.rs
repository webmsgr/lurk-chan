use std::str::FromStr;

use common::{Report, ReportStatus};
use sqlx::{SqlitePool, sqlite::{SqlitePoolOptions, SqliteConnectOptions}, migrate};


#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Database migration error: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),
    #[error("Invalid report status: {0}")]
    InvalidReportStatus(String)
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
            report_reason: self.report_status,
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
            },,
        })
    }
}