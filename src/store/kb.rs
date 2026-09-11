use sqlx::{SqlitePool, query, query_as, query_scalar};

use crate::infra::error::AppResult;
use crate::models::{KbBook, KbNode, KbTreeNode};

const KB_BOOK_COLS: &str =
    "id, title, slug, summary, cover_url, status, visibility, sort_order, created_at, updated_at";
const KB_NODE_COLS: &str =
    "id, book_id, parent_id, node_type, title, slug, sort_order, content_md, content_html,
                status, visibility, published_at, created_at, updated_at";

pub async fn list_kb_books(pool: &SqlitePool) -> AppResult<Vec<KbBook>> {
    let sql = format!("SELECT {KB_BOOK_COLS} FROM kb_books ORDER BY sort_order ASC, id ASC");
    Ok(query_as::<_, KbBook>(&sql).fetch_all(pool).await?)
}

pub async fn list_public_kb_books(pool: &SqlitePool, logged_in: bool) -> AppResult<Vec<KbBook>> {
    if logged_in {
        return list_kb_books(pool).await;
    }
    let sql = format!(
        "SELECT {KB_BOOK_COLS} FROM kb_books
         WHERE status = 'published' AND visibility = 'public'
         ORDER BY sort_order ASC, id ASC"
    );
    Ok(query_as::<_, KbBook>(&sql).fetch_all(pool).await?)
}

pub async fn find_kb_book_by_id(pool: &SqlitePool, id: i64) -> AppResult<Option<KbBook>> {
    let sql = format!("SELECT {KB_BOOK_COLS} FROM kb_books WHERE id = ?");
    Ok(query_as::<_, KbBook>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn find_kb_book_by_slug(pool: &SqlitePool, slug: &str) -> AppResult<Option<KbBook>> {
    let sql = format!("SELECT {KB_BOOK_COLS} FROM kb_books WHERE slug = ?");
    Ok(query_as::<_, KbBook>(&sql)
        .bind(slug)
        .fetch_optional(pool)
        .await?)
}

pub async fn kb_book_slug_taken(
    pool: &SqlitePool,
    slug: &str,
    exclude_id: Option<i64>,
) -> AppResult<bool> {
    let taken: Option<i64> = if let Some(id) = exclude_id {
        query_scalar("SELECT id FROM kb_books WHERE slug = ? AND id != ? LIMIT 1")
            .bind(slug)
            .bind(id)
            .fetch_optional(pool)
            .await?
    } else {
        query_scalar("SELECT id FROM kb_books WHERE slug = ? LIMIT 1")
            .bind(slug)
            .fetch_optional(pool)
            .await?
    };
    Ok(taken.is_some())
}

pub async fn next_kb_book_sort(pool: &SqlitePool) -> AppResult<i64> {
    let max: Option<i64> = query_scalar("SELECT MAX(sort_order) FROM kb_books")
        .fetch_one(pool)
        .await?;
    Ok(max.unwrap_or(-1) + 1)
}

pub async fn create_kb_book(
    pool: &SqlitePool,
    title: &str,
    slug: &str,
    summary: &str,
    cover_url: &str,
    status: &str,
    visibility: &str,
    now: &str,
) -> AppResult<i64> {
    let sort_order = next_kb_book_sort(pool).await?;
    let result = query(
        "INSERT INTO kb_books (title, slug, summary, cover_url, status, visibility, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(title)
    .bind(slug)
    .bind(summary)
    .bind(cover_url)
    .bind(status)
    .bind(visibility)
    .bind(sort_order)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_kb_book(
    pool: &SqlitePool,
    id: i64,
    title: &str,
    slug: &str,
    summary: &str,
    cover_url: &str,
    status: &str,
    visibility: &str,
    now: &str,
) -> AppResult<()> {
    query(
        "UPDATE kb_books SET title = ?, slug = ?, summary = ?, cover_url = ?, status = ?, visibility = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(title)
    .bind(slug)
    .bind(summary)
    .bind(cover_url)
    .bind(status)
    .bind(visibility)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_kb_book(pool: &SqlitePool, id: i64) -> AppResult<()> {
    query("DELETE FROM kb_books WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_kb_nodes_by_book(pool: &SqlitePool, book_id: i64) -> AppResult<Vec<KbNode>> {
    let sql = format!(
        "SELECT {KB_NODE_COLS} FROM kb_nodes WHERE book_id = ? ORDER BY sort_order ASC, id ASC"
    );
    Ok(query_as::<_, KbNode>(&sql)
        .bind(book_id)
        .fetch_all(pool)
        .await?)
}

pub async fn find_kb_node_by_id(pool: &SqlitePool, id: i64) -> AppResult<Option<KbNode>> {
    let sql = format!("SELECT {KB_NODE_COLS} FROM kb_nodes WHERE id = ?");
    Ok(query_as::<_, KbNode>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn find_kb_node_by_book_slug(
    pool: &SqlitePool,
    book_id: i64,
    slug: &str,
) -> AppResult<Option<KbNode>> {
    let sql = format!("SELECT {KB_NODE_COLS} FROM kb_nodes WHERE book_id = ? AND slug = ?");
    Ok(query_as::<_, KbNode>(&sql)
        .bind(book_id)
        .bind(slug)
        .fetch_optional(pool)
        .await?)
}

pub async fn kb_node_slug_taken(
    pool: &SqlitePool,
    book_id: i64,
    slug: &str,
    exclude_id: Option<i64>,
) -> AppResult<bool> {
    let taken: Option<i64> = if let Some(id) = exclude_id {
        query_scalar("SELECT id FROM kb_nodes WHERE book_id = ? AND slug = ? AND id != ? LIMIT 1")
            .bind(book_id)
            .bind(slug)
            .bind(id)
            .fetch_optional(pool)
            .await?
    } else {
        query_scalar("SELECT id FROM kb_nodes WHERE book_id = ? AND slug = ? LIMIT 1")
            .bind(book_id)
            .bind(slug)
            .fetch_optional(pool)
            .await?
    };
    Ok(taken.is_some())
}

pub async fn next_kb_node_sort(
    pool: &SqlitePool,
    book_id: i64,
    parent_id: Option<i64>,
) -> AppResult<i64> {
    let max: Option<i64> = match parent_id {
        Some(pid) => {
            query_scalar("SELECT MAX(sort_order) FROM kb_nodes WHERE book_id = ? AND parent_id = ?")
                .bind(book_id)
                .bind(pid)
                .fetch_one(pool)
                .await?
        }
        None => {
            query_scalar(
                "SELECT MAX(sort_order) FROM kb_nodes WHERE book_id = ? AND parent_id IS NULL",
            )
            .bind(book_id)
            .fetch_one(pool)
            .await?
        }
    };
    Ok(max.unwrap_or(-1) + 1)
}

pub async fn create_kb_node(
    pool: &SqlitePool,
    book_id: i64,
    parent_id: Option<i64>,
    node_type: &str,
    title: &str,
    slug: &str,
    content_md: &str,
    content_html: &str,
    status: &str,
    visibility: &str,
    published_at: Option<&str>,
    now: &str,
) -> AppResult<i64> {
    let sort_order = next_kb_node_sort(pool, book_id, parent_id).await?;
    let result = query(
        "INSERT INTO kb_nodes
         (book_id, parent_id, node_type, title, slug, sort_order, content_md, content_html,
          status, visibility, published_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(book_id)
    .bind(parent_id)
    .bind(node_type)
    .bind(title)
    .bind(slug)
    .bind(sort_order)
    .bind(content_md)
    .bind(content_html)
    .bind(status)
    .bind(visibility)
    .bind(published_at)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_kb_node(
    pool: &SqlitePool,
    id: i64,
    title: &str,
    slug: &str,
    content_md: &str,
    content_html: &str,
    status: &str,
    visibility: &str,
    published_at: Option<&str>,
    now: &str,
) -> AppResult<()> {
    query(
        "UPDATE kb_nodes SET title = ?, slug = ?, content_md = ?, content_html = ?,
         status = ?, visibility = ?, published_at = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(title)
    .bind(slug)
    .bind(content_md)
    .bind(content_html)
    .bind(status)
    .bind(visibility)
    .bind(published_at)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn rename_kb_node(pool: &SqlitePool, id: i64, title: &str, now: &str) -> AppResult<()> {
    query("UPDATE kb_nodes SET title = ?, updated_at = ? WHERE id = ?")
        .bind(title)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_kb_node(pool: &SqlitePool, id: i64) -> AppResult<()> {
    query("DELETE FROM kb_nodes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn move_kb_node(
    pool: &SqlitePool,
    node_id: i64,
    new_parent_id: Option<i64>,
    before_id: Option<i64>,
    now: &str,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    let sql = format!("SELECT {KB_NODE_COLS} FROM kb_nodes WHERE id = ?");
    let Some(node) = query_as::<_, KbNode>(&sql)
        .bind(node_id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Err(crate::infra::error::AppError::not_found("页面不存在"));
    };

    if new_parent_id == Some(node_id) {
        return Err(crate::infra::error::AppError::bad_request("不能移动到自身"));
    }

    if let Some(pid) = new_parent_id {
        let Some(parent) = query_as::<_, KbNode>(&sql)
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await?
        else {
            return Err(crate::infra::error::AppError::bad_request("父节点不存在"));
        };
        if parent.book_id != node.book_id {
            return Err(crate::infra::error::AppError::bad_request(
                "父节点不属于该知识库",
            ));
        }

        let mut cursor = Some(pid);
        while let Some(cid) = cursor {
            if cid == node_id {
                return Err(crate::infra::error::AppError::bad_request(
                    "不能移动到自己的子目录中",
                ));
            }
            cursor = query_scalar::<_, Option<i64>>("SELECT parent_id FROM kb_nodes WHERE id = ?")
                .bind(cid)
                .fetch_optional(&mut *tx)
                .await?
                .flatten();
        }
    }

    if let Some(bid) = before_id {
        if bid == node_id {
            return Err(crate::infra::error::AppError::bad_request("无效的插入位置"));
        }
        let Some(before) = query_as::<_, KbNode>(&sql)
            .bind(bid)
            .fetch_optional(&mut *tx)
            .await?
        else {
            return Err(crate::infra::error::AppError::bad_request("插入位置不存在"));
        };
        if before.book_id != node.book_id {
            return Err(crate::infra::error::AppError::bad_request(
                "插入位置不属于该知识库",
            ));
        }
        if before.parent_id != new_parent_id {
            return Err(crate::infra::error::AppError::bad_request(
                "插入位置与目标父节点不一致",
            ));
        }
    }

    let old_parent_id = node.parent_id;
    let book_id = node.book_id;

    if old_parent_id == new_parent_id {
        let siblings = list_sibling_ids(&mut tx, book_id, new_parent_id).await?;
        let cur = siblings.iter().position(|id| *id == node_id);
        let target = match before_id {
            Some(bid) => siblings.iter().position(|id| *id == bid),
            None => Some(siblings.len()),
        };
        if let (Some(c), Some(t)) = (cur, target) {
            let desired = if t > c { t - 1 } else { t };
            if c == desired {
                tx.rollback().await?;
                return Ok(());
            }
        }
    }

    query("UPDATE kb_nodes SET parent_id = ?, updated_at = ? WHERE id = ?")
        .bind(new_parent_id)
        .bind(now)
        .bind(node_id)
        .execute(&mut *tx)
        .await?;

    let mut target_ids = list_sibling_ids(&mut tx, book_id, new_parent_id).await?;
    target_ids.retain(|id| *id != node_id);
    let insert_at = match before_id {
        Some(bid) => target_ids.iter().position(|id| *id == bid).unwrap_or(target_ids.len()),
        None => target_ids.len(),
    };
    target_ids.insert(insert_at, node_id);
    write_sort_orders(&mut tx, &target_ids).await?;

    if old_parent_id != new_parent_id {
        let old_ids = list_sibling_ids(&mut tx, book_id, old_parent_id).await?;
        write_sort_orders(&mut tx, &old_ids).await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn list_sibling_ids(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    book_id: i64,
    parent_id: Option<i64>,
) -> AppResult<Vec<i64>> {
    Ok(match parent_id {
        Some(pid) => {
            query_scalar(
                "SELECT id FROM kb_nodes WHERE book_id = ? AND parent_id = ? ORDER BY sort_order ASC, id ASC",
            )
            .bind(book_id)
            .bind(pid)
            .fetch_all(&mut **tx)
            .await?
        }
        None => {
            query_scalar(
                "SELECT id FROM kb_nodes WHERE book_id = ? AND parent_id IS NULL ORDER BY sort_order ASC, id ASC",
            )
            .bind(book_id)
            .fetch_all(&mut **tx)
            .await?
        }
    })
}

async fn write_sort_orders(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ids: &[i64],
) -> AppResult<()> {
    for (i, id) in ids.iter().enumerate() {
        query("UPDATE kb_nodes SET sort_order = ? WHERE id = ?")
            .bind(i as i64)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

pub fn build_kb_tree(nodes: &[KbNode]) -> Vec<KbTreeNode> {
    fn to_tree(n: &KbNode, all: &[KbNode]) -> KbTreeNode {
        let children = all
            .iter()
            .filter(|c| c.parent_id == Some(n.id))
            .map(|c| to_tree(c, all))
            .collect();
        KbTreeNode {
            id: n.id,
            parent_id: n.parent_id,
            node_type: n.node_type.clone(),
            title: n.title.clone(),
            slug: n.slug.clone(),
            status: n.status.clone(),
            visibility: n.visibility.clone(),
            updated_at: n.updated_at.clone(),
            children,
        }
    }

    nodes
        .iter()
        .filter(|n| n.parent_id.is_none())
        .map(|n| to_tree(n, nodes))
        .collect()
}

pub fn build_public_kb_tree(nodes: &[KbNode], logged_in: bool) -> Vec<KbTreeNode> {
    fn visible_doc(n: &KbNode, logged_in: bool) -> bool {
        logged_in || n.is_publicly_visible()
    }

    fn to_tree(n: &KbNode, all: &[KbNode], logged_in: bool) -> Option<KbTreeNode> {
        let children: Vec<KbTreeNode> = all
            .iter()
            .filter(|c| c.parent_id == Some(n.id))
            .filter_map(|c| to_tree(c, all, logged_in))
            .collect();
        if n.is_folder() {
            if children.is_empty() && !logged_in {
                return None;
            }
        } else if !visible_doc(n, logged_in) {
            return None;
        }
        Some(KbTreeNode {
            id: n.id,
            parent_id: n.parent_id,
            node_type: n.node_type.clone(),
            title: n.title.clone(),
            slug: n.slug.clone(),
            status: n.status.clone(),
            visibility: n.visibility.clone(),
            updated_at: n.updated_at.clone(),
            children,
        })
    }

    nodes
        .iter()
        .filter(|n| n.parent_id.is_none())
        .filter_map(|n| to_tree(n, nodes, logged_in))
        .collect()
}

pub fn kb_tree_stats(tree: &[KbTreeNode], nodes: &[KbNode]) -> (usize, usize) {
    fn walk(nodes: &[KbTreeNode], all: &[KbNode], docs: &mut usize, words: &mut usize) {
        for n in nodes {
            if n.is_doc() {
                *docs += 1;
                if let Some(raw) = all.iter().find(|x| x.id == n.id) {
                    *words += raw
                        .content_md
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .count();
                }
            }
            walk(&n.children, all, docs, words);
        }
    }
    let mut docs = 0;
    let mut words = 0;
    walk(tree, nodes, &mut docs, &mut words);
    (docs, words)
}

pub fn kb_breadcrumbs(
    nodes: &[KbNode],
    node: &KbNode,
    book_slug: &str,
) -> Vec<crate::models::KbCrumb> {
    use crate::models::KbCrumb;
    let mut crumbs = Vec::new();
    let mut cur = node.parent_id;
    let mut guard = 0;
    while let Some(pid) = cur {
        guard += 1;
        if guard > 64 {
            break;
        }
        let Some(p) = nodes.iter().find(|n| n.id == pid) else {
            break;
        };
        crumbs.push(KbCrumb {
            title: p.title.clone(),
            href: Some(format!("/docs/{}/{}", book_slug, p.slug)),
        });
        cur = p.parent_id;
    }
    crumbs.reverse();
    crumbs
}

pub fn kb_page_neighbors(
    tree: &[KbTreeNode],
    current_id: i64,
) -> (Option<(String, String)>, Option<(String, String)>) {
    fn flatten(nodes: &[KbTreeNode], out: &mut Vec<(i64, String, String)>) {
        for n in nodes {
            if n.is_doc() && !n.slug.is_empty() {
                out.push((n.id, n.title.clone(), n.slug.clone()));
            }
            flatten(&n.children, out);
        }
    }
    let mut pages = Vec::new();
    flatten(tree, &mut pages);
    let Some(i) = pages.iter().position(|(id, _, _)| *id == current_id) else {
        return (None, None);
    };
    let prev = i
        .checked_sub(1)
        .map(|j| (pages[j].1.clone(), pages[j].2.clone()));
    let next = if i + 1 < pages.len() {
        Some((pages[i + 1].1.clone(), pages[i + 1].2.clone()))
    } else {
        None
    };
    (prev, next)
}
