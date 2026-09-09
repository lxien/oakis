use sqlx::{SqlitePool, query, query_as, query_scalar};

use crate::infra::error::AppResult;
use crate::models::{Taxonomy, TaxonomyCount};

pub async fn count_taxonomies(pool: &SqlitePool, scope: &str, kind: &str) -> AppResult<i64> {
    Ok(
        query_scalar("SELECT COUNT(*) FROM taxonomies WHERE scope = ? AND kind = ?")
            .bind(scope)
            .bind(kind)
            .fetch_one(pool)
            .await?,
    )
}

pub async fn list_taxonomies(
    pool: &SqlitePool,
    scope: &str,
    kind: &str,
) -> AppResult<Vec<Taxonomy>> {
    let rows = query_as::<_, Taxonomy>(
        "SELECT id, scope, kind, name, slug, created_at
         FROM taxonomies
         WHERE scope = ? AND kind = ?
         ORDER BY name COLLATE NOCASE ASC",
    )
    .bind(scope)
    .bind(kind)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_taxonomies_page(
    pool: &SqlitePool,
    scope: &str,
    kind: &str,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Taxonomy>> {
    let rows = query_as::<_, Taxonomy>(
        "SELECT id, scope, kind, name, slug, created_at
         FROM taxonomies
         WHERE scope = ? AND kind = ?
         ORDER BY name COLLATE NOCASE ASC
         LIMIT ? OFFSET ?",
    )
    .bind(scope)
    .bind(kind)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_categories_with_counts(
    pool: &SqlitePool,
    include_private: bool,
) -> AppResult<Vec<TaxonomyCount>> {
    list_taxonomy_counts(
        pool,
        Taxonomy::KIND_CATEGORY,
        include_private,
        "t.name COLLATE NOCASE ASC",
    )
    .await
}

pub async fn list_tags_with_counts(
    pool: &SqlitePool,
    include_private: bool,
) -> AppResult<Vec<TaxonomyCount>> {
    list_taxonomy_counts(
        pool,
        Taxonomy::KIND_TAG,
        include_private,
        "post_count DESC, t.name COLLATE NOCASE ASC",
    )
    .await
}

async fn list_taxonomy_counts(
    pool: &SqlitePool,
    kind: &str,
    include_private: bool,
    order_by: &str,
) -> AppResult<Vec<TaxonomyCount>> {
    let visibility = if include_private {
        "p.status = 'published'"
    } else {
        "p.status = 'published' AND p.visibility = 'public'"
    };
    let sql = format!(
        "SELECT t.name, t.slug,
                COUNT(p.id) AS post_count
         FROM taxonomies t
         LEFT JOIN post_taxonomies pt ON pt.taxonomy_id = t.id
         LEFT JOIN posts p ON p.id = pt.post_id AND {visibility}
         WHERE t.scope = ? AND t.kind = ?
         GROUP BY t.id, t.name, t.slug
         HAVING COUNT(p.id) > 0
         ORDER BY {order_by}"
    );
    let rows = query_as::<_, TaxonomyCount>(&sql)
        .bind(Taxonomy::SCOPE_POST)
        .bind(kind)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn create_taxonomy(
    pool: &SqlitePool,
    scope: &str,
    kind: &str,
    name: &str,
    slug: &str,
) -> AppResult<i64> {
    let res = query("INSERT INTO taxonomies (scope, kind, name, slug) VALUES (?, ?, ?, ?)")
        .bind(scope)
        .bind(kind)
        .bind(name)
        .bind(slug)
        .execute(pool)
        .await?;
    Ok(res.last_insert_rowid())
}

pub async fn find_taxonomy_by_id(pool: &SqlitePool, id: i64) -> AppResult<Option<Taxonomy>> {
    let row = query_as::<_, Taxonomy>(
        "SELECT id, scope, kind, name, slug, created_at FROM taxonomies WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn update_taxonomy(pool: &SqlitePool, id: i64, name: &str, slug: &str) -> AppResult<()> {
    query("UPDATE taxonomies SET name = ?, slug = ? WHERE id = ?")
        .bind(name)
        .bind(slug)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_taxonomy(pool: &SqlitePool, id: i64) -> AppResult<()> {
    query("DELETE FROM taxonomies WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_taxonomy_by_slug(
    pool: &SqlitePool,
    scope: &str,
    kind: &str,
    slug: &str,
) -> AppResult<Option<Taxonomy>> {
    let row = query_as::<_, Taxonomy>(
        "SELECT id, scope, kind, name, slug, created_at
         FROM taxonomies WHERE scope = ? AND kind = ? AND slug = ?",
    )
    .bind(scope)
    .bind(kind)
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
