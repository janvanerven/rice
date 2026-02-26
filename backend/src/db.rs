use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::time::Duration;

pub struct DbPools {
    pub write: SqlitePool,
    pub read: SqlitePool,
}

impl DbPools {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let base_opts = SqliteConnectOptions::from_str(database_url)?
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
            .pragma("synchronous", "NORMAL")
            .pragma("foreign_keys", "ON")
            .create_if_missing(true);

        let write = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(base_opts.clone())
            .await?;

        let read = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(base_opts.read_only(true))
            .await?;

        sqlx::migrate!("./migrations").run(&write).await?;

        tracing::info!("Database connected and migrated");

        Ok(DbPools { write, read })
    }
}
