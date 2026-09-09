use sqlx::{FromRow, SqlitePool, query_as, query_scalar};

use crate::infra::error::AppResult;

const RECENT_LIMIT: i64 = 5;
const PENDING_LIMIT: i64 = 5;
const TITLE_MAX: usize = 48;

#[derive(Debug, Clone)]
pub struct DashItem {
    pub kind: String,
    pub id: i64,
    pub book_id: Option<i64>,
    pub title: String,
    pub updated_at: String,
    pub status: String,
}

impl DashItem {
    pub fn kind_label(&self) -> &'static str {
        match self.kind.as_str() {
            "post" => "文章",
            "page" => "页面",
            "doc" => "文档",
            "spark" => "灵感",
            _ => "内容",
        }
    }

    pub fn href(&self) -> String {
        match self.kind.as_str() {
            "post" => format!("/admin/posts/{}/edit", self.id),
            "page" => format!("/admin/pages/{}/edit", self.id),
            "doc" => match self.book_id {
                Some(bid) => format!("/admin/docs/{bid}?id={}", self.id),
                None => "/admin/docs".into(),
            },
            "spark" => format!("/admin/sparks/{}/edit", self.id),
            _ => "/admin".into(),
        }
    }

    pub fn when(&self) -> String {
        crate::infra::timefmt::format_local(&self.updated_at)
    }

    pub fn is_draft(&self) -> bool {
        self.status == "draft"
    }
}

#[derive(Debug, Clone)]
pub struct DashboardData {
    pub posts_published: i64,
    pub posts_draft: i64,
    pub page_count: i64,
    pub book_count: i64,
    pub spark_count: i64,
    pub media_count: i64,
    pub pending: Vec<DashItem>,
    pub recent: Vec<DashItem>,
}

impl DashboardData {
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn has_recent(&self) -> bool {
        !self.recent.is_empty()
    }

    pub fn posts_total(&self) -> i64 {
        self.posts_published + self.posts_draft
    }
}

#[derive(Debug, FromRow)]
struct DashRow {
    kind: String,
    id: i64,
    book_id: Option<i64>,
    title: String,
    updated_at: String,
    status: String,
}

fn clip_title(raw: &str) -> String {
    let text = raw.trim();
    if text.is_empty() {
        return "（无标题）".into();
    }
    let line = text.lines().next().unwrap_or(text).trim();
    let mut out = String::new();
    for (i, ch) in line.chars().enumerate() {
        if i >= TITLE_MAX {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    if out.is_empty() {
        "（无标题）".into()
    } else {
        out
    }
}

fn from_row(row: DashRow) -> DashItem {
    DashItem {
        kind: row.kind,
        id: row.id,
        book_id: row.book_id,
        title: clip_title(&row.title),
        updated_at: row.updated_at,
        status: row.status,
    }
}

async fn count_scalar(pool: &SqlitePool, sql: &str) -> AppResult<i64> {
    Ok(query_scalar::<_, i64>(sql).fetch_one(pool).await?)
}

pub async fn load_dashboard(pool: &SqlitePool) -> AppResult<DashboardData> {
    let (
        posts_published,
        posts_draft,
        page_count,
        book_count,
        spark_count,
        media_count,
        pending,
        recent,
    ) = tokio::try_join!(
        count_scalar(
            pool,
            "SELECT COUNT(*) FROM posts WHERE status = 'published'"
        ),
        count_scalar(pool, "SELECT COUNT(*) FROM posts WHERE status = 'draft'"),
        count_scalar(pool, "SELECT COUNT(*) FROM pages"),
        count_scalar(pool, "SELECT COUNT(*) FROM kb_books"),
        count_scalar(pool, "SELECT COUNT(*) FROM sparks"),
        count_scalar(pool, "SELECT COUNT(*) FROM media"),
        list_pending(pool),
        list_recent(pool),
    )?;

    Ok(DashboardData {
        posts_published,
        posts_draft,
        page_count,
        book_count,
        spark_count,
        media_count,
        pending,
        recent,
    })
}

async fn list_pending(pool: &SqlitePool) -> AppResult<Vec<DashItem>> {
    let rows = query_as::<_, DashRow>(
        "SELECT kind, id, book_id, title, updated_at, status FROM (
            SELECT 'post' AS kind, id, CAST(NULL AS INTEGER) AS book_id, title, updated_at, status
            FROM posts WHERE status = 'draft'
            UNION ALL
            SELECT 'page', id, NULL, title, updated_at, status
            FROM pages WHERE status = 'draft'
            UNION ALL
            SELECT 'doc', id, book_id, title, updated_at, status
            FROM kb_nodes WHERE node_type = 'doc' AND status = 'draft'
         )
         ORDER BY updated_at DESC
         LIMIT ?",
    )
    .bind(PENDING_LIMIT)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(from_row).collect())
}

async fn list_recent(pool: &SqlitePool) -> AppResult<Vec<DashItem>> {
    let rows = query_as::<_, DashRow>(
        "SELECT kind, id, book_id, title, updated_at, status FROM (
            SELECT 'post' AS kind, id, CAST(NULL AS INTEGER) AS book_id, title, updated_at, status
            FROM posts
            UNION ALL
            SELECT 'page', id, NULL, title, updated_at, status
            FROM pages
            UNION ALL
            SELECT 'doc', id, book_id, title, updated_at, status
            FROM kb_nodes WHERE node_type = 'doc'
            UNION ALL
            SELECT 'spark', id, NULL, content_md, updated_at, ''
            FROM sparks
         )
         ORDER BY updated_at DESC
         LIMIT ?",
    )
    .bind(RECENT_LIMIT)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(from_row).collect())
}
