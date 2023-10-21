use sqlx::{SqlitePool, SqliteConnection, pool::PoolConnection, Sqlite, Acquire, Executor};
use serenity::prelude::TypeMapKey;
use std::sync::Arc;



pub type DBConn = SqliteConnection;


pub struct LurkChan {
    db: SqlitePool
}

impl LurkChan {
    pub async fn db(&self) -> DBConn {
       self.db.acquire().await.expect("Failed to get database! this is an extremely bad error!").detach()
    }
    pub fn new(db: SqlitePool) -> Self {
        Self {
            db
        }
    }
}



impl TypeMapKey for LurkChan {
    type Value = Arc<LurkChan>;
}