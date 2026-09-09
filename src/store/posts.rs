use sqlx::{SqlitePool, query, query_as, query_scalar};

use crate::infra::error::AppResult;
use crate::models::{Post, PostView, Taxonomy};

pub async fn count_published_posts(pool: &SqlitePool, include_private: bool) -> AppResult<i64> {
    let sql = if include_private {
        "SELECT COUNT(*) FROM posts WHERE status = 'published'"
    } else {
        "SELECT COUNT(*) FROM posts WHERE status = 'published' AND visibility = 'public'"
    };
    Ok(query_scalar(sql).fetch_one(pool).await?)
}

pub async fn list_published_posts(
    pool: &SqlitePool,
    include_private: bool,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Post>> {
    let sql = if include_private {
        "SELECT id, title, slug, summary, content_md, content_html, status, visibility, cover_url,
                published_at, created_at, updated_at
         FROM posts
         WHERE status = 'published'
         ORDER BY COALESCE(published_at, created_at) DESC, id DESC
         LIMIT ? OFFSET ?"
    } else {
        "SELECT id, title, slug, summary, content_md, content_html, status, visibility, cover_url,
                published_at, created_at, updated_at
         FROM posts
         WHERE status = 'published' AND visibility = 'public'
         ORDER BY COALESCE(published_at, created_at) DESC, id DESC
         LIMIT ? OFFSET ?"
    };
    let mut posts = query_as::<_, Post>(sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    for post in &mut posts {
        post.sanitize_cover();
    }
    Ok(posts)
}

pub async fn count_all_posts(pool: &SqlitePool) -> AppResult<i64> {
    Ok(query_scalar("SELECT COUNT(*) FROM posts")
        .fetch_one(pool)
        .await?)
}

pub async fn list_all_posts(pool: &SqlitePool, limit: i64, offset: i64) -> AppResult<Vec<Post>> {
    let mut posts = query_as::<_, Post>(
        "SELECT id, title, slug, summary, content_md, content_html, status, visibility, cover_url,
                published_at, created_at, updated_at
         FROM posts
         ORDER BY updated_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    for post in &mut posts {
        post.sanitize_cover();
    }
    Ok(posts)
}

pub async fn find_post_by_slug(pool: &SqlitePool, slug: &str) -> AppResult<Option<Post>> {
    let mut post = query_as::<_, Post>(
        "SELECT id, title, slug, summary, content_md, content_html, status, visibility, cover_url,
                published_at, created_at, updated_at
         FROM posts WHERE slug = ?",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    if let Some(post) = post.as_mut() {
        post.sanitize_cover();
    }
    Ok(post)
}

pub async fn find_post_by_id(pool: &SqlitePool, id: i64) -> AppResult<Option<Post>> {
    let mut post = query_as::<_, Post>(
        "SELECT id, title, slug, summary, content_md, content_html, status, visibility, cover_url,
                published_at, created_at, updated_at
         FROM posts WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    if let Some(post) = post.as_mut() {
        post.sanitize_cover();
    }
    Ok(post)
}

pub async fn list_taxonomies_for_post(pool: &SqlitePool, post_id: i64) -> AppResult<Vec<Taxonomy>> {
    let rows = query_as::<_, Taxonomy>(
        "SELECT t.id, t.scope, t.kind, t.name, t.slug, t.created_at
         FROM taxonomies t
         INNER JOIN post_taxonomies pt ON pt.taxonomy_id = t.id
         WHERE pt.post_id = ?
         ORDER BY t.kind ASC, t.name COLLATE NOCASE ASC",
    )
    .bind(post_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn set_post_taxonomies(
    pool: &SqlitePool,
    post_id: i64,
    taxonomy_ids: &[i64],
) -> AppResult<()> {
    query("DELETE FROM post_taxonomies WHERE post_id = ?")
        .bind(post_id)
        .execute(pool)
        .await?;
    for id in taxonomy_ids {
        query("INSERT OR IGNORE INTO post_taxonomies (post_id, taxonomy_id) VALUES (?, ?)")
            .bind(post_id)
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn set_post_relations(
    pool: &SqlitePool,
    post_id: i64,
    related_ids: &[i64],
) -> AppResult<()> {
    use crate::models::RelatedPostRef;

    query("DELETE FROM post_relations WHERE post_id = ?")
        .bind(post_id)
        .execute(pool)
        .await?;

    let mut seen = std::collections::HashSet::new();
    let mut order = 0i64;
    for &rid in related_ids {
        if rid == post_id || !seen.insert(rid) {
            continue;
        }
        if order >= RelatedPostRef::MANUAL_MAX as i64 {
            break;
        }
        if find_post_by_id(pool, rid).await?.is_none() {
            continue;
        }
        query(
            "INSERT INTO post_relations (post_id, related_post_id, sort_order)
             VALUES (?, ?, ?)",
        )
        .bind(post_id)
        .bind(rid)
        .bind(order)
        .execute(pool)
        .await?;
        order += 1;
    }
    Ok(())
}

pub async fn list_related_ids(pool: &SqlitePool, post_id: i64) -> AppResult<Vec<i64>> {
    let rows: Vec<(i64,)> = query_as(
        "SELECT related_post_id FROM post_relations
         WHERE post_id = ?
         ORDER BY sort_order ASC, related_post_id ASC",
    )
    .bind(post_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

pub async fn list_related_refs_ordered(
    pool: &SqlitePool,
    ids: &[i64],
) -> AppResult<Vec<crate::models::RelatedPostRef>> {
    use crate::models::RelatedPostRef;

    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut map = std::collections::HashMap::new();
    for &id in ids {
        if let Some(post) = find_post_by_id(pool, id).await? {
            map.insert(id, RelatedPostRef::from_post(&post));
        }
    }
    Ok(ids.iter().filter_map(|id| map.remove(id)).collect())
}

fn post_visibility_sql(include_private: bool, alias: &str) -> String {
    if include_private {
        format!("{alias}.status = 'published'")
    } else {
        format!("{alias}.status = 'published' AND {alias}.visibility = 'public'")
    }
}

pub async fn resolve_related_posts(
    pool: &SqlitePool,
    post_id: i64,
    include_private: bool,
    limit: usize,
) -> AppResult<Vec<crate::models::RelatedPostRef>> {
    use crate::models::RelatedPostRef;

    if limit == 0 {
        return Ok(Vec::new());
    }

    let manual_ids = list_related_ids(pool, post_id).await?;
    let mut out: Vec<RelatedPostRef> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    seen.insert(post_id);

    for id in manual_ids {
        if out.len() >= limit {
            break;
        }
        let Some(post) = find_post_by_id(pool, id).await? else {
            continue;
        };
        if !post.is_published() {
            continue;
        }
        if !include_private && post.is_private() {
            continue;
        }
        if !seen.insert(post.id) {
            continue;
        }
        out.push(RelatedPostRef::from_post(&post));
    }

    if out.len() < limit {
        let need = limit - out.len();
        let auto = suggest_related_posts(pool, post_id, include_private, &seen, need).await?;
        for p in auto {
            if seen.insert(p.id) {
                out.push(p);
                if out.len() >= limit {
                    break;
                }
            }
        }
    }

    Ok(out)
}

async fn suggest_related_posts(
    pool: &SqlitePool,
    post_id: i64,
    include_private: bool,
    exclude: &std::collections::HashSet<i64>,
    limit: usize,
) -> AppResult<Vec<crate::models::RelatedPostRef>> {
    use crate::models::RelatedPostRef;

    if limit == 0 {
        return Ok(Vec::new());
    }

    let vis = post_visibility_sql(include_private, "p");
    let exclude_ids: Vec<i64> = exclude.iter().copied().collect();
    let exclude_sql = if exclude_ids.is_empty() {
        "0".to_string()
    } else {
        exclude_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };

    let scored_sql = format!(
        "SELECT p.id, p.title, p.slug, p.summary, p.content_md, p.content_html, p.status, p.visibility,
                p.cover_url, p.published_at, p.created_at, p.updated_at
         FROM posts p
         INNER JOIN post_taxonomies pt ON pt.post_id = p.id
         INNER JOIN taxonomies t ON t.id = pt.taxonomy_id
         WHERE {vis}
           AND p.id != ?
           AND p.id NOT IN ({exclude_sql})
           AND pt.taxonomy_id IN (
             SELECT taxonomy_id FROM post_taxonomies WHERE post_id = ?
           )
         GROUP BY p.id
         ORDER BY SUM(CASE WHEN t.kind = 'tag' THEN 3 ELSE 1 END) DESC,
                  COALESCE(p.published_at, p.created_at) DESC
         LIMIT ?"
    );

    let mut posts = query_as::<_, Post>(&scored_sql)
        .bind(post_id)
        .bind(post_id)
        .bind(limit as i64)
        .fetch_all(pool)
        .await?;
    for post in &mut posts {
        post.sanitize_cover();
    }

    let mut out: Vec<RelatedPostRef> = posts.iter().map(RelatedPostRef::from_post).collect();
    if out.len() >= limit {
        out.truncate(limit);
        return Ok(out);
    }

    let mut seen: std::collections::HashSet<i64> = exclude.iter().copied().collect();
    for r in &out {
        seen.insert(r.id);
    }
    let need = limit - out.len();
    let exclude_sql2 = if seen.is_empty() {
        "0".to_string()
    } else {
        seen.iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    let recent_sql = format!(
        "SELECT p.id, p.title, p.slug, p.summary, p.content_md, p.content_html, p.status, p.visibility,
                p.cover_url, p.published_at, p.created_at, p.updated_at
         FROM posts p
         WHERE {vis}
           AND p.id != ?
           AND p.id NOT IN ({exclude_sql2})
         ORDER BY COALESCE(p.published_at, p.created_at) DESC, p.id DESC
         LIMIT ?"
    );
    let mut recent = query_as::<_, Post>(&recent_sql)
        .bind(post_id)
        .bind(need as i64)
        .fetch_all(pool)
        .await?;
    for post in &mut recent {
        post.sanitize_cover();
    }
    for post in recent {
        out.push(RelatedPostRef::from_post(&post));
    }
    Ok(out)
}

pub async fn count_admin_posts_search(
    pool: &SqlitePool,
    q: &str,
    exclude_id: Option<i64>,
) -> AppResult<i64> {
    let q = q.trim();
    if q.is_empty() {
        if let Some(ex) = exclude_id {
            return Ok(query_scalar("SELECT COUNT(*) FROM posts WHERE id != ?")
                .bind(ex)
                .fetch_one(pool)
                .await?);
        }
        return count_all_posts(pool).await;
    }
    let escaped = q
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    if let Some(ex) = exclude_id {
        Ok(query_scalar(
            "SELECT COUNT(*) FROM posts
             WHERE id != ?
               AND (title LIKE ? ESCAPE '\\' OR slug LIKE ? ESCAPE '\\')",
        )
        .bind(ex)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(pool)
        .await?)
    } else {
        Ok(query_scalar(
            "SELECT COUNT(*) FROM posts
             WHERE title LIKE ? ESCAPE '\\' OR slug LIKE ? ESCAPE '\\'",
        )
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(pool)
        .await?)
    }
}

pub async fn list_admin_posts_search(
    pool: &SqlitePool,
    q: &str,
    exclude_id: Option<i64>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Post>> {
    let q = q.trim();
    let mut posts = if q.is_empty() {
        if let Some(ex) = exclude_id {
            query_as::<_, Post>(
                "SELECT id, title, slug, summary, content_md, content_html, status, visibility, cover_url,
                        published_at, created_at, updated_at
                 FROM posts WHERE id != ?
                 ORDER BY updated_at DESC
                 LIMIT ? OFFSET ?",
            )
            .bind(ex)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        } else {
            list_all_posts(pool, limit, offset).await?
        }
    } else {
        let escaped = q
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        if let Some(ex) = exclude_id {
            query_as::<_, Post>(
                "SELECT id, title, slug, summary, content_md, content_html, status, visibility, cover_url,
                        published_at, created_at, updated_at
                 FROM posts
                 WHERE id != ?
                   AND (title LIKE ? ESCAPE '\\' OR slug LIKE ? ESCAPE '\\')
                 ORDER BY updated_at DESC
                 LIMIT ? OFFSET ?",
            )
            .bind(ex)
            .bind(&pattern)
            .bind(&pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        } else {
            query_as::<_, Post>(
                "SELECT id, title, slug, summary, content_md, content_html, status, visibility, cover_url,
                        published_at, created_at, updated_at
                 FROM posts
                 WHERE title LIKE ? ESCAPE '\\' OR slug LIKE ? ESCAPE '\\'
                 ORDER BY updated_at DESC
                 LIMIT ? OFFSET ?",
            )
            .bind(&pattern)
            .bind(&pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
    };
    for post in &mut posts {
        post.sanitize_cover();
    }
    Ok(posts)
}

pub async fn to_post_view(pool: &SqlitePool, post: Post) -> AppResult<PostView> {
    let all = list_taxonomies_for_post(pool, post.id).await?;
    let mut categories = Vec::new();
    let mut tags = Vec::new();
    for t in all {
        if t.is_category() {
            categories.push(t);
        } else if t.is_tag() {
            tags.push(t);
        }
    }
    Ok(PostView {
        post,
        categories,
        tags,
    })
}

pub async fn to_post_views(pool: &SqlitePool, posts: Vec<Post>) -> AppResult<Vec<PostView>> {
    let mut out = Vec::with_capacity(posts.len());
    for post in posts {
        out.push(to_post_view(pool, post).await?);
    }
    Ok(out)
}

pub async fn count_uncategorized_published_posts(
    pool: &SqlitePool,
    include_private: bool,
) -> AppResult<i64> {
    let sql = if include_private {
        "SELECT COUNT(*)
         FROM posts p
         WHERE p.status = 'published'
           AND NOT EXISTS (
             SELECT 1 FROM post_taxonomies pt
             INNER JOIN taxonomies t ON t.id = pt.taxonomy_id
             WHERE pt.post_id = p.id AND t.kind = 'category'
           )"
    } else {
        "SELECT COUNT(*)
         FROM posts p
         WHERE p.status = 'published' AND p.visibility = 'public'
           AND NOT EXISTS (
             SELECT 1 FROM post_taxonomies pt
             INNER JOIN taxonomies t ON t.id = pt.taxonomy_id
             WHERE pt.post_id = p.id AND t.kind = 'category'
           )"
    };
    Ok(query_scalar(sql).fetch_one(pool).await?)
}

pub async fn list_uncategorized_published_posts(
    pool: &SqlitePool,
    include_private: bool,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Post>> {
    let sql = if include_private {
        "SELECT p.id, p.title, p.slug, p.summary, p.content_md, p.content_html, p.status, p.visibility, p.cover_url,
                p.published_at, p.created_at, p.updated_at
         FROM posts p
         WHERE p.status = 'published'
           AND NOT EXISTS (
             SELECT 1 FROM post_taxonomies pt
             INNER JOIN taxonomies t ON t.id = pt.taxonomy_id
             WHERE pt.post_id = p.id AND t.kind = 'category'
           )
         ORDER BY COALESCE(p.published_at, p.created_at) DESC, p.id DESC
         LIMIT ? OFFSET ?"
    } else {
        "SELECT p.id, p.title, p.slug, p.summary, p.content_md, p.content_html, p.status, p.visibility, p.cover_url,
                p.published_at, p.created_at, p.updated_at
         FROM posts p
         WHERE p.status = 'published' AND p.visibility = 'public'
           AND NOT EXISTS (
             SELECT 1 FROM post_taxonomies pt
             INNER JOIN taxonomies t ON t.id = pt.taxonomy_id
             WHERE pt.post_id = p.id AND t.kind = 'category'
           )
         ORDER BY COALESCE(p.published_at, p.created_at) DESC, p.id DESC
         LIMIT ? OFFSET ?"
    };
    let mut posts = query_as::<_, Post>(sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    for post in &mut posts {
        post.sanitize_cover();
    }
    Ok(posts)
}

pub async fn count_published_posts_by_taxonomy(
    pool: &SqlitePool,
    taxonomy_id: i64,
    include_private: bool,
) -> AppResult<i64> {
    let sql = if include_private {
        "SELECT COUNT(*)
         FROM posts p
         INNER JOIN post_taxonomies pt ON pt.post_id = p.id
         WHERE pt.taxonomy_id = ? AND p.status = 'published'"
    } else {
        "SELECT COUNT(*)
         FROM posts p
         INNER JOIN post_taxonomies pt ON pt.post_id = p.id
         WHERE pt.taxonomy_id = ? AND p.status = 'published' AND p.visibility = 'public'"
    };
    Ok(query_scalar(sql).bind(taxonomy_id).fetch_one(pool).await?)
}

pub async fn list_published_posts_by_taxonomy(
    pool: &SqlitePool,
    taxonomy_id: i64,
    include_private: bool,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Post>> {
    let sql = if include_private {
        "SELECT p.id, p.title, p.slug, p.summary, p.content_md, p.content_html, p.status, p.visibility, p.cover_url,
                p.published_at, p.created_at, p.updated_at
         FROM posts p
         INNER JOIN post_taxonomies pt ON pt.post_id = p.id
         WHERE pt.taxonomy_id = ? AND p.status = 'published'
         ORDER BY COALESCE(p.published_at, p.created_at) DESC, p.id DESC
         LIMIT ? OFFSET ?"
    } else {
        "SELECT p.id, p.title, p.slug, p.summary, p.content_md, p.content_html, p.status, p.visibility, p.cover_url,
                p.published_at, p.created_at, p.updated_at
         FROM posts p
         INNER JOIN post_taxonomies pt ON pt.post_id = p.id
         WHERE pt.taxonomy_id = ? AND p.status = 'published' AND p.visibility = 'public'
         ORDER BY COALESCE(p.published_at, p.created_at) DESC, p.id DESC
         LIMIT ? OFFSET ?"
    };
    let mut posts = query_as::<_, Post>(sql)
        .bind(taxonomy_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    for post in &mut posts {
        post.sanitize_cover();
    }
    Ok(posts)
}

pub async fn post_slug_taken(
    pool: &SqlitePool,
    slug: &str,
    exclude_id: Option<i64>,
) -> AppResult<bool> {
    let count: i64 = if let Some(id) = exclude_id {
        query_scalar("SELECT COUNT(*) FROM posts WHERE slug = ? AND id != ?")
            .bind(slug)
            .bind(id)
            .fetch_one(pool)
            .await?
    } else {
        query_scalar("SELECT COUNT(*) FROM posts WHERE slug = ?")
            .bind(slug)
            .fetch_one(pool)
            .await?
    };
    Ok(count > 0)
}

pub async fn create_post(
    pool: &SqlitePool,
    title: &str,
    slug: &str,
    summary: &str,
    content_md: &str,
    content_html: &str,
    status: &str,
    visibility: &str,
    cover_url: Option<&str>,
    published_at: Option<&str>,
    created_at: &str,
    updated_at: &str,
) -> Result<i64, sqlx::Error> {
    let r = query(
        "INSERT INTO posts (title, slug, summary, content_md, content_html, status, visibility, cover_url, published_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(title)
    .bind(slug)
    .bind(summary)
    .bind(content_md)
    .bind(content_html)
    .bind(status)
    .bind(visibility)
    .bind(cover_url)
    .bind(published_at)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;
    Ok(r.last_insert_rowid())
}

pub async fn update_post(
    pool: &SqlitePool,
    id: i64,
    title: &str,
    slug: &str,
    summary: &str,
    content_md: &str,
    content_html: &str,
    status: &str,
    visibility: &str,
    cover_url: Option<&str>,
    published_at: &str,
    updated_at: &str,
) -> Result<(), sqlx::Error> {
    query(
        "UPDATE posts
         SET title = ?, slug = ?, summary = ?, content_md = ?, content_html = ?,
             status = ?, visibility = ?, cover_url = ?, published_at = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(title)
    .bind(slug)
    .bind(summary)
    .bind(content_md)
    .bind(content_html)
    .bind(status)
    .bind(visibility)
    .bind(cover_url)
    .bind(published_at)
    .bind(updated_at)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_post(pool: &SqlitePool, id: i64) -> AppResult<()> {
    query("DELETE FROM posts WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
