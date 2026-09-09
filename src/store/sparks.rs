use sqlx::{SqlitePool, query, query_as, query_scalar};

use crate::infra::error::AppResult;
use crate::models::Spark;

pub async fn count_public_sparks(pool: &SqlitePool) -> AppResult<i64> {
    Ok(
        query_scalar("SELECT COUNT(*) FROM sparks WHERE visibility = 'public'")
            .fetch_one(pool)
            .await?,
    )
}

pub async fn list_public_sparks(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Spark>> {
    let rows = query_as::<_, Spark>(
        "SELECT id, content_md, content_html, visibility, created_at, updated_at
         FROM sparks WHERE visibility = 'public'
         ORDER BY created_at DESC, id DESC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn count_all_sparks(pool: &SqlitePool) -> AppResult<i64> {
    Ok(query_scalar("SELECT COUNT(*) FROM sparks")
        .fetch_one(pool)
        .await?)
}

pub async fn list_all_sparks(pool: &SqlitePool, limit: i64, offset: i64) -> AppResult<Vec<Spark>> {
    let rows = query_as::<_, Spark>(
        "SELECT id, content_md, content_html, visibility, created_at, updated_at
         FROM sparks
         ORDER BY created_at DESC, id DESC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_spark_by_id(pool: &SqlitePool, id: i64) -> AppResult<Option<Spark>> {
    let row = query_as::<_, Spark>(
        "SELECT id, content_md, content_html, visibility, created_at, updated_at
         FROM sparks WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn create_spark(
    pool: &SqlitePool,
    content_md: &str,
    content_html: &str,
    visibility: &str,
    now: &str,
) -> AppResult<i64> {
    let r = query(
        "INSERT INTO sparks (content_md, content_html, visibility, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(content_md)
    .bind(content_html)
    .bind(visibility)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(r.last_insert_rowid())
}

pub async fn update_spark(
    pool: &SqlitePool,
    id: i64,
    content_md: &str,
    content_html: &str,
    visibility: &str,
    now: &str,
) -> AppResult<()> {
    query(
        "UPDATE sparks SET content_md = ?, content_html = ?, visibility = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(content_md)
    .bind(content_html)
    .bind(visibility)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_spark(pool: &SqlitePool, id: i64) -> AppResult<()> {
    query("DELETE FROM sparks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
