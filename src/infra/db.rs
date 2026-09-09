use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{SqlitePool, query_scalar};
use std::str::FromStr;

pub async fn connect(database_url: &str) -> anyhow::Result<SqlitePool> {
    use sqlx::sqlite::SqliteJournalMode;

    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

pub async fn is_installed(pool: &SqlitePool) -> anyhow::Result<bool> {
    let flag: Option<String> = query_scalar("SELECT value FROM settings WHERE key = 'installed'")
        .fetch_optional(pool)
        .await?;
    if flag.as_deref() == Some("1") {
        return Ok(true);
    }

    let count: i64 = query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(count > 0)
}
