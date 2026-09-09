use sqlx::{SqlitePool, query, query_as, query_scalar};

use crate::infra::error::AppResult;
use crate::models::{NavPageRef, Page};

pub async fn list_published_page_refs(pool: &SqlitePool) -> AppResult<Vec<NavPageRef>> {
    let rows: Vec<(i64, String, String, String)> = query_as(
        "SELECT id, title, slug, visibility FROM pages
         WHERE status = 'published'
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, title, slug, visibility)| NavPageRef {
            id,
            title,
            slug,
            visibility,
        })
        .collect())
}

pub async fn count_all_pages(pool: &SqlitePool) -> AppResult<i64> {
    Ok(query_scalar("SELECT COUNT(*) FROM pages")
        .fetch_one(pool)
        .await?)
}

pub async fn list_all_pages(pool: &SqlitePool, limit: i64, offset: i64) -> AppResult<Vec<Page>> {
    let rows = query_as::<_, Page>(
        "SELECT id, title, slug, content_md, status, visibility, show_in_nav, created_at, updated_at
         FROM pages ORDER BY id ASC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_page_by_id(pool: &SqlitePool, id: i64) -> AppResult<Option<Page>> {
    let page = query_as::<_, Page>(
        "SELECT id, title, slug, content_md, status, visibility, show_in_nav, created_at, updated_at
         FROM pages WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(page)
}

pub async fn find_page_by_slug(pool: &SqlitePool, slug: &str) -> AppResult<Option<Page>> {
    let page = query_as::<_, Page>(
        "SELECT id, title, slug, content_md, status, visibility, show_in_nav, created_at, updated_at
         FROM pages WHERE slug = ?",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    Ok(page)
}

pub async fn page_slug_taken(
    pool: &SqlitePool,
    slug: &str,
    exclude_id: Option<i64>,
) -> AppResult<bool> {
    let count: i64 = if let Some(id) = exclude_id {
        query_scalar("SELECT COUNT(*) FROM pages WHERE slug = ? AND id != ?")
            .bind(slug)
            .bind(id)
            .fetch_one(pool)
            .await?
    } else {
        query_scalar("SELECT COUNT(*) FROM pages WHERE slug = ?")
            .bind(slug)
            .fetch_one(pool)
            .await?
    };
    Ok(count > 0)
}

pub async fn create_page(
    pool: &SqlitePool,
    title: &str,
    slug: &str,
    content_md: &str,
    status: &str,
    visibility: &str,
    created_at: &str,
    updated_at: &str,
) -> Result<(), sqlx::Error> {
    query(
        "INSERT INTO pages (title, slug, content_md, status, visibility, show_in_nav, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(title)
    .bind(slug)
    .bind(content_md)
    .bind(status)
    .bind(visibility)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_page(
    pool: &SqlitePool,
    id: i64,
    title: &str,
    slug: &str,
    content_md: &str,
    status: &str,
    visibility: &str,
    updated_at: &str,
) -> Result<(), sqlx::Error> {
    query(
        "UPDATE pages SET title = ?, slug = ?, content_md = ?, status = ?, visibility = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(title)
    .bind(slug)
    .bind(content_md)
    .bind(status)
    .bind(visibility)
    .bind(updated_at)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_page(pool: &SqlitePool, id: i64) -> AppResult<()> {
    query("DELETE FROM pages WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn purge_page_from_nav(pool: &SqlitePool, page_id: i64) -> AppResult<()> {
    let settings = super::settings::load_settings(pool).await?;
    let before = settings.nav_items.len();
    let next: Vec<_> = settings
        .nav_items
        .into_iter()
        .filter(|item| item.page_id != Some(page_id))
        .collect();
    if next.len() != before {
        super::settings::save_nav_items(pool, &next).await?;
    }
    Ok(())
}
