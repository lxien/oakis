use sqlx::{SqlitePool, query_as, query_scalar};

use crate::infra::error::AppResult;

pub async fn count_search_hits(
    pool: &SqlitePool,
    q: &str,
    include_private: bool,
) -> AppResult<i64> {
    let escaped = q
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    let sql = if include_private {
        "SELECT COUNT(*) FROM (
            SELECT p.id AS id
            FROM posts p
            WHERE p.status = 'published'
              AND (p.title LIKE ? ESCAPE '\\'
                   OR p.summary LIKE ? ESCAPE '\\'
                   OR p.content_md LIKE ? ESCAPE '\\')
            UNION ALL
            SELECT pg.id
            FROM pages pg
            WHERE pg.status = 'published'
              AND (pg.title LIKE ? ESCAPE '\\'
                   OR pg.content_md LIKE ? ESCAPE '\\')
            UNION ALL
            SELECT n.id
            FROM kb_nodes n
            INNER JOIN kb_books b ON b.id = n.book_id
            WHERE n.node_type = 'doc'
              AND n.status = 'published'
              AND b.status = 'published'
              AND (n.title LIKE ? ESCAPE '\\'
                   OR n.content_md LIKE ? ESCAPE '\\'
                   OR b.title LIKE ? ESCAPE '\\')
         )"
    } else {
        "SELECT COUNT(*) FROM (
            SELECT p.id AS id
            FROM posts p
            WHERE p.status = 'published' AND p.visibility = 'public'
              AND (p.title LIKE ? ESCAPE '\\'
                   OR p.summary LIKE ? ESCAPE '\\'
                   OR p.content_md LIKE ? ESCAPE '\\')
            UNION ALL
            SELECT pg.id
            FROM pages pg
            WHERE pg.status = 'published' AND pg.visibility = 'public'
              AND (pg.title LIKE ? ESCAPE '\\'
                   OR pg.content_md LIKE ? ESCAPE '\\')
            UNION ALL
            SELECT n.id
            FROM kb_nodes n
            INNER JOIN kb_books b ON b.id = n.book_id
            WHERE n.node_type = 'doc'
              AND n.status = 'published' AND n.visibility = 'public'
              AND b.status = 'published' AND b.visibility = 'public'
              AND (n.title LIKE ? ESCAPE '\\'
                   OR n.content_md LIKE ? ESCAPE '\\'
                   OR b.title LIKE ? ESCAPE '\\')
         )"
    };
    Ok(query_scalar(sql)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(pool)
        .await?)
}

pub async fn search_hits(
    pool: &SqlitePool,
    q: &str,
    include_private: bool,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<crate::models::SearchHit>> {
    let escaped = q
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    let sql = if include_private {
        "SELECT kind, title, href, excerpt, sort_at FROM (
            SELECT 'post' AS kind,
                   p.title AS title,
                   '/posts/' || p.slug AS href,
                   CASE
                     WHEN TRIM(p.summary) != '' THEN p.summary
                     ELSE substr(p.content_md, 1, 160)
                   END AS excerpt,
                   COALESCE(p.published_at, p.created_at) AS sort_at
            FROM posts p
            WHERE p.status = 'published'
              AND (p.title LIKE ? ESCAPE '\\'
                   OR p.summary LIKE ? ESCAPE '\\'
                   OR p.content_md LIKE ? ESCAPE '\\')
            UNION ALL
            SELECT 'page',
                   pg.title,
                   '/p/' || pg.slug,
                   substr(pg.content_md, 1, 160),
                   pg.updated_at
            FROM pages pg
            WHERE pg.status = 'published'
              AND (pg.title LIKE ? ESCAPE '\\'
                   OR pg.content_md LIKE ? ESCAPE '\\')
            UNION ALL
            SELECT 'doc',
                   n.title,
                   '/docs/' || b.slug || '/' || n.slug,
                   CASE
                     WHEN TRIM(n.content_md) != '' THEN substr(n.content_md, 1, 160)
                     ELSE b.title
                   END,
                   COALESCE(n.published_at, n.updated_at)
            FROM kb_nodes n
            INNER JOIN kb_books b ON b.id = n.book_id
            WHERE n.node_type = 'doc'
              AND n.status = 'published'
              AND b.status = 'published'
              AND (n.title LIKE ? ESCAPE '\\'
                   OR n.content_md LIKE ? ESCAPE '\\'
                   OR b.title LIKE ? ESCAPE '\\')
         )
         ORDER BY sort_at DESC
         LIMIT ? OFFSET ?"
    } else {
        "SELECT kind, title, href, excerpt, sort_at FROM (
            SELECT 'post' AS kind,
                   p.title AS title,
                   '/posts/' || p.slug AS href,
                   CASE
                     WHEN TRIM(p.summary) != '' THEN p.summary
                     ELSE substr(p.content_md, 1, 160)
                   END AS excerpt,
                   COALESCE(p.published_at, p.created_at) AS sort_at
            FROM posts p
            WHERE p.status = 'published' AND p.visibility = 'public'
              AND (p.title LIKE ? ESCAPE '\\'
                   OR p.summary LIKE ? ESCAPE '\\'
                   OR p.content_md LIKE ? ESCAPE '\\')
            UNION ALL
            SELECT 'page',
                   pg.title,
                   '/p/' || pg.slug,
                   substr(pg.content_md, 1, 160),
                   pg.updated_at
            FROM pages pg
            WHERE pg.status = 'published' AND pg.visibility = 'public'
              AND (pg.title LIKE ? ESCAPE '\\'
                   OR pg.content_md LIKE ? ESCAPE '\\')
            UNION ALL
            SELECT 'doc',
                   n.title,
                   '/docs/' || b.slug || '/' || n.slug,
                   CASE
                     WHEN TRIM(n.content_md) != '' THEN substr(n.content_md, 1, 160)
                     ELSE b.title
                   END,
                   COALESCE(n.published_at, n.updated_at)
            FROM kb_nodes n
            INNER JOIN kb_books b ON b.id = n.book_id
            WHERE n.node_type = 'doc'
              AND n.status = 'published' AND n.visibility = 'public'
              AND b.status = 'published' AND b.visibility = 'public'
              AND (n.title LIKE ? ESCAPE '\\'
                   OR n.content_md LIKE ? ESCAPE '\\'
                   OR b.title LIKE ? ESCAPE '\\')
         )
         ORDER BY sort_at DESC
         LIMIT ? OFFSET ?"
    };
    let rows = query_as::<_, crate::models::SearchHit>(sql)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}
