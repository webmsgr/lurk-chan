use std::{str::FromStr, path::PathBuf};

use common::{Action, Location, Report, ReportStatus};
use sqlx::{
    migrate,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool, error::DatabaseError,
};

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
    CommonError(#[from] common::Error),
    #[error("Action not found: {0}")]
    ActionNotFound(u32),
    #[error("Foreign Key Error: {0}")]
    ForeignKeyError(String)
}

pub struct  Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self, Error> {
        let options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename("lurk_chan.db");
        let pool = SqlitePoolOptions::new().connect_with(options).await?;
        migrate!().run(&pool).await?;
        let db = Self { pool };
        db.vacuum().await?;
        Ok(db)
    }
    pub async fn vacuum(&self) -> Result<(), Error> {
        sqlx::query("vacuum;").execute(&self.pool).await?;
        Ok(())
    }
    pub async fn optimize(&self) -> Result<(), Error> {
        sqlx::query("PRAGMA optimize;").execute(&self.pool).await?;
        Ok(())
    }
    pub async fn backup_to(&self, huh: PathBuf) -> Result<(), Error> {
        sqlx::query(&format!("vacuum into '{}';", huh.display()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn get_report_from_id(&self, report_id: u32) -> Result<Option<Report>, Error> {
        let report_id = report_id as i64;
        let res: Option<DBReport> =
            sqlx::query_as!(DBReport, "select * from Reports where id = ?", report_id)
                .fetch_optional(&self.pool)
                .await?;
        match res {
            Some(i) => Ok(Some(i.into_report()?)),
            None => Ok(None),
        }
    }
    pub async fn add_report(&self, report: Report) -> Result<u32, Error> {
        let r = DBReport::from_report(report);
        let res =
            sqlx::query!(
                "insert into Reports(reporter_id, reporter_name, reported_id, reported_name, report_reason, report_status, server, time, claimant, location) values (?,?,?,?,?,?,?,?,?,?)", 
                r.reporter_id,
                r.reporter_name,
                r.reported_id,
                r.reported_name,
                r.report_reason,
                r.report_status,
                r.server,
                r.time,
                r.claimant,
                r.location
            ).execute(&self.pool).await?;
        Ok(res.last_insert_rowid() as u32)
    }
    pub async fn get_action_from_id(&self, id: u32) -> Result<Option<Action>, Error> {
        let action_id = id as i64;
        let res: Option<DBAction> =
            sqlx::query_as!(DBAction, "select * from Actions where id = ?", action_id)
                .fetch_optional(&self.pool)
                .await?;
        match res {
            Some(i) => Ok(Some(i.try_into()?)),
            None => Ok(None),
        }
    }
    pub async fn add_report_message(
        &self,
        channel_id: u64,
        message_id: u64,
        report_id: u32,
    ) -> Result<(), Error> {
        let (a, b, c) = (
            report_id as i64,
            channel_id.to_string(),
            message_id.to_string(),
        );
        sqlx::query!(
            "insert into ReportMessages(report_id, channel, message) values (?,?,?)",
            a,
            b,
            c
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn add_action_message(
        &self,
        channel_id: u64,
        message_id: u64,
        action_id: u32,
    ) -> Result<(), Error> {
        let (a, b, c) = (
            action_id as i64,
            channel_id.to_string(),
            message_id.to_string(),
        );
        sqlx::query!(
            "insert into ActionMessages(action_id, channel, message) values (?,?,?)",
            a,
            b,
            c
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn get_report_message(&self, id: u32) -> Result<Option<(u64, u64)>, Error> {
        let id = id as i64;
        let res: Option<(String, String)> = sqlx::query!(
            "select channel, message from ReportMessages where report_id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map(|i| i.map(|i| (i.channel, i.message)))?;
        match res {
            Some((a, b)) => Ok(Some((a.parse()?, b.parse()?))),
            None => Ok(None),
        }
    }
    pub async fn get_action_message(&self, id: u32) -> Result<Option<(u64, u64)>, Error> {
        let id = id as i64;
        let res: Option<(String, String)> = sqlx::query!(
            "select channel, message from ActionMessages where action_id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map(|i| i.map(|i| (i.channel, i.message)))?;
        match res {
            Some((a, b)) => Ok(Some((a.parse()?, b.parse()?))),
            None => Ok(None),
        }
    }
    pub async fn audit_count_from_server(&self, server: Location) -> Result<u32, Error> {
        let s = server.to_string();
        let res: i32 = sqlx::query_scalar!("select count(*) from Actions where server = ?", s).fetch_one(&self.pool).await?;
        Ok(res as u32)
    }
    pub async fn audit_count_without_report(&self) -> Result<u32, Error> {
        let res: i32 = sqlx::query_scalar!("select count(*) from Actions where report is null").fetch_one(&self.pool).await?;
        Ok(res as u32)
    }
    pub async fn get_action_message_from_report_id(
        &self,
        id: u32,
    ) -> Result<Option<(u64, u64)>, Error> {
        let id = id as i64;
        let res: Option<(String, String)> = sqlx::query!("select channel, message from ActionMessages where action_id = (select id from Actions where report = ?)", id)
            .fetch_optional(&self.pool).await.map(|i| i.map(|i| (i.channel, i.message)))?;
        match res {
            Some((a, b)) => Ok(Some((a.parse()?, b.parse()?))),
            None => Ok(None),
        }
    }
    pub async fn get_report_count(&self, id: &str) -> Result<u32, Error> {
        let res: i32 =
            sqlx::query_scalar!("select count(*) from Reports where reported_id = ?", id)
                .fetch_one(&self.pool)
                .await?;
        Ok(res as u32)
    }
    pub async fn leaderboard_reports(&self, limit: u32) -> Result<Vec<(u64, u32)>, Error> {
        let res = sqlx::query!(
            "select claimant, count(*) as count from Reports where claimant is not null group by claimant order by count desc limit ?",
            limit
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|i| (i.claimant, i.count));
        Ok(res.map(|(a, b)| (a.expect("claimant is not null").parse().unwrap(), b as u32)).collect())
    }
    pub async fn all_reports_with_status(&self, status: ReportStatus) -> Result<Vec<(u32, Report)>, Error> {
        let s = status.to_db();
        let res = sqlx::query_as!(
            DBReport,
            "select * from Reports where report_status = ?",
            s
        ).fetch_all(&self.pool).await?;
        Ok(res.into_iter().map(|i| (i.id.unwrap() as u32, i.into_report().unwrap())).collect())
    }
pub async fn expire_report(&self, rid: u32) -> Result<(), Error> {
        let rid = rid as i64;
        sqlx::query!("update Reports set report_status = 'expired' where id = ?", rid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn leaderboard_audit(&self, limit: u32) -> Result<Vec<(u64, u32)>, Error> {
        let res = sqlx::query!(
            "select claimant, count(*) as count from Actions where claimant is not null group by claimant order by count desc limit ?",
            limit
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|i| (i.claimant, i.count));
        Ok(res.map(|(a, b)| (a.parse().unwrap(), b as u32)).collect())
    }
    pub async fn claim_report(&self, id: u32, claimant: u64) -> Result<(), Error> {
        let id = id as i64;
        let c = claimant.to_string();
        sqlx::query!(
            "update Reports set report_status = 'claimed', claimant = ? where id = ?",
            c,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn who_claimed_report(&self, id: u32) -> Result<Option<u64>, Error> {
        let id = id as i64;
        let res: Option<String> =
            sqlx::query_scalar!("select claimant from Reports where id = ?", id)
                .fetch_optional(&self.pool)
                .await?
                .flatten();
        match res {
            Some(i) => Ok(Some(i.parse()?)),
            None => Ok(None),
        }
    }
    pub async fn add_action(&self, action: Action) -> Result<u32, Error> {
        let a = DBAction::from(action);
        let res = 
            sqlx::query!(
                "insert into Actions(target_id, target_username, offense, action, server, claimant, report) values (?,?,?,?,?,?,?)", 
                a.target_id,
                a.target_username,
                a.offense,
                a.action,
                a.server,
                a.claimant,
                a.report
            ).execute(&self.pool).await?;
        Ok(res.last_insert_rowid() as u32)
    }
    pub async fn close_report(&self, id: u32) -> Result<(), Error> {
        let id = id as i64;
        sqlx::query!(
            "update Reports set report_status = 'closed' where id = ?",
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn edit_action(
        &self,
        id: u32,
        audit: Action,
        now: String,
        who: u64,
    ) -> Result<(), Error> {
        let old = self
            .get_action_from_id(id)
            .await?
            .ok_or_else(|| Error::ActionNotFound(id))?;
        let old_val = serde_json::to_value(&old).expect("should never fail");
        let new_val = serde_json::to_value(&audit).expect("should never fail");
        let diff = json_patch::diff(&old_val, &new_val);

        let id = id as i64;
        let old_str = serde_json::to_string(&old).expect("should never fail");
        let new_str = serde_json::to_string(&audit).expect("should never fail");
        let diff_str = serde_json::to_string(&diff).expect("should never fail");
        sqlx::query!("update Actions set target_id = ?, target_username = ?, offense = ?, action = ? where id = ?", 
                audit.target_id,
                audit.target_username,
                audit.offense,
                audit.action,
                id).execute(&self.pool).await?;
        let who_str = who.to_string();
        sqlx::query!(
            "insert into AuditEdits(action_id, old, new, who, time, changes) values (?,?,?,?,?,?)",
            id,
            old_str,
            new_str,
            who_str,
            now,
            diff_str
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn collect_user_info(&self, user: &str) -> Result<UserInfo, Error> {
        const LIMIT: i32 = 10;
        let (times_reported, 
            preview_reported, 
            times_reported_other, 
            preview_reported_others, 
            times_actioned,
        preview_actioned) = tokio::try_join!(
            sqlx::query_scalar!("select count(*) from Reports where reported_id = ?", user).fetch_one(&self.pool),
            sqlx::query_as!(DBReport, "select * from Reports where reported_id = ? order by time desc limit ?", user, LIMIT).fetch_all(&self.pool),
            sqlx::query_scalar!("select count(*) from Reports where reporter_id = ?", user).fetch_one(&self.pool),
            sqlx::query_as!(DBReport, "select * from Reports where reporter_id = ? order by time desc limit ?", user, LIMIT).fetch_all(&self.pool),
            sqlx::query_scalar!("select count(*) from Actions where claimant = ?", user).fetch_one(&self.pool),
            sqlx::query_as!(DBAction, "select * from Actions where claimant = ? order by id desc limit ?", user, LIMIT).fetch_all(&self.pool),
        )?;
        Ok(
            UserInfo { 
                times_reported: times_reported as u32, 
                preview_reported : preview_reported.into_iter().map(|i| (i.id.expect("database") as u32, i.into_report().unwrap())).collect(), 
                times_reported_others: times_reported_other as u32, 
                preview_reported_others: preview_reported_others.into_iter().map(|i| (i.id.expect("database") as u32, i.into_report().unwrap())).collect(), 
                times_actioned: times_actioned as u32, 
                preview_actioned: preview_actioned.into_iter().map(|i| (i.id.expect("database") as u32, i.try_into().unwrap())).collect(), 
            }
        )
    }
    pub async fn total_report_count(&self) -> Result<u32, Error> {
        let res: i32 = sqlx::query_scalar!("select count(*) from Reports").fetch_one(&self.pool).await?;
        Ok(res as u32)
    }
    pub async fn total_action_count(&self) -> Result<u32, Error> {
        let res: i32 = sqlx::query_scalar!("select count(*) from Actions").fetch_one(&self.pool).await?;
        Ok(res as u32)
    }
    pub async fn get_report_count_by_status(&self, status: ReportStatus) -> Result<u32, Error> {
        let s = status.to_db();
        let res: i32 = sqlx::query_scalar!("select count(*) from Reports where report_status = ?", s).fetch_one(&self.pool).await?;
        Ok(res as u32)
    }
    pub async fn get_report_message_count(&self) -> Result<u32, Error> {
        let res: i32 = sqlx::query_scalar!("select count(*) from ReportMessages").fetch_one(&self.pool).await?;
        Ok(res as u32)
    }
    pub async fn get_action_message_count(&self) -> Result<u32, Error> {
        let res: i32 = sqlx::query_scalar!("select count(*) from ActionMessages").fetch_one(&self.pool).await?;
        Ok(res as u32)
    }
    pub async fn foreign_key_check(&self) -> Result<usize, Error> {
        let s = sqlx::query!("pragma foreign_key_check;").fetch_all(&self.pool).await?;
        Ok(s.len())
    }
    pub async fn integrety_check(&self) -> Result<(), Error> {
        let s = sqlx::query!("pragma integrity_check;").fetch_all(&self.pool).await?;
        for i in s {
            if i.integrity_check != "ok" {
                return Err(Error::ForeignKeyError(i.integrity_check))
            }
        }
        Ok(())
    }
}


pub struct UserInfo {
    /// how many times has this user been reported in total?
    pub times_reported: u32,
    /// contains the last 10 reports against this user
    pub preview_reported: Vec<(u32, Report)>,
    /// how many times has this user reported someone else in total?
    pub times_reported_others: u32,
    /// contains the last 10 reports against others by this user
    pub preview_reported_others: Vec<(u32, Report)>,
    /// how many times has this user been actioned in total?
    pub times_actioned: u32,
    /// contains the last 10 actions taken against this user
    pub preview_actioned: Vec<(u32, Action)>,
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
    location: String,
}

impl DBReport {
    fn into_report(self) -> Result<Report, Error> {
        Ok(Report {
            reporter_id: self.reporter_id,
            reporter_name: self.reporter_name,
            reported_id: self.reported_id,
            reported_name: self.reported_name,
            report_reason: self.report_reason,
            report_status: ReportStatus::from_db(&self.report_status)
                .ok_or_else(|| Error::InvalidReportStatus(self.report_status))?,
            server: self.server,
            time: self.time,
            claimant: match self.claimant {
                Some(i) => Some(i.parse()?),
                None => None,
            },
            location: Location::from_str(self.location.as_str())?,
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
            location: r.location.to_string(),
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
            claimant: value.claimant.to_string(),
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
            server: Location::from_str(&self.server)?,
        })
    }
}
