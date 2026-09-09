use sqlx::{SqlitePool, query, query_as, query_scalar};

use crate::infra::error::AppResult;
use crate::models::Media;

pub async fn count_media(pool: &SqlitePool) -> AppResult<i64> {
    Ok(query_scalar("SELECT COUNT(*) FROM media")
        .fetch_one(pool)
        .await?)
}

pub async fn count_media_images(pool: &SqlitePool) -> AppResult<i64> {
    Ok(
        query_scalar("SELECT COUNT(*) FROM media WHERE mime LIKE 'image/%'")
            .fetch_one(pool)
            .await?,
    )
}

pub async fn count_media_picker(pool: &SqlitePool, images_only: bool) -> AppResult<i64> {
    if images_only {
        count_media_images(pool).await
    } else {
        count_media(pool).await
    }
}

pub async fn list_media(pool: &SqlitePool, limit: i64, offset: i64) -> AppResult<Vec<Media>> {
    let rows = query_as::<_, Media>(
        "SELECT id, name, url, mime, size, created_at
         FROM media
         ORDER BY created_at DESC, id DESC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_media_images(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Media>> {
    let rows = query_as::<_, Media>(
        "SELECT id, name, url, mime, size, created_at
         FROM media
         WHERE mime LIKE 'image/%'
         ORDER BY created_at DESC, id DESC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_media_picker(
    pool: &SqlitePool,
    images_only: bool,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Media>> {
    if images_only {
        list_media_images(pool, limit, offset).await
    } else {
        list_media(pool, limit, offset).await
    }
}

pub async fn find_media_by_id(pool: &SqlitePool, id: i64) -> AppResult<Option<Media>> {
    let row = query_as::<_, Media>(
        "SELECT id, name, url, mime, size, created_at FROM media WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn insert_media(
    pool: &SqlitePool,
    name: &str,
    url: &str,
    mime: &str,
    size: i64,
) -> AppResult<i64> {
    let r = query("INSERT INTO media (name, url, mime, size) VALUES (?, ?, ?, ?)")
        .bind(name)
        .bind(url)
        .bind(mime)
        .bind(size)
        .execute(pool)
        .await?;
    Ok(r.last_insert_rowid())
}

pub async fn delete_media_row(pool: &SqlitePool, id: i64) -> AppResult<()> {
    query("DELETE FROM media WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
