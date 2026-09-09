use sqlx::{SqlitePool, query, query_as, query_scalar};

use crate::infra::error::AppResult;
use crate::models::User;

pub async fn count_users<'e, E>(executor: E) -> AppResult<i64>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    Ok(query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(executor)
        .await?)
}

pub async fn create_user<'e, E>(
    executor: E,
    username: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
        .bind(username)
        .bind(password_hash)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn find_user_by_username(pool: &SqlitePool, username: &str) -> AppResult<Option<User>> {
    let user = query_as::<_, User>(
        "SELECT id, username, password_hash, created_at FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

pub async fn find_user_by_id(pool: &SqlitePool, id: i64) -> AppResult<Option<User>> {
    let user = query_as::<_, User>(
        "SELECT id, username, password_hash, created_at FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

pub async fn update_user_password(
    pool: &SqlitePool,
    user_id: i64,
    password_hash: &str,
) -> AppResult<()> {
    query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(password_hash)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
