use axum::extract::{Path, State};
use axum::response::Html;
use tower_sessions::Session;

use crate::infra::error::{AppError, AppResult, render};
use crate::infra::markdown::render_markdown;
use crate::infra::state::AppState;
use crate::models::{KbBook, KbNeighbor, KbTreeNode};
use crate::store::{
    build_public_kb_tree, find_kb_book_by_slug, find_kb_node_by_book_slug, kb_breadcrumbs,
    kb_page_neighbors, kb_tree_stats, list_kb_nodes_by_book, list_public_kb_books,
};
use crate::views::{DocsBookTemplate, DocsHomeTemplate};
use crate::web::load_public_shell;

use super::{catalog_html, html_escape, tree_html};

pub async fn docs_home(State(state): State<AppState>, session: Session) -> AppResult<Html<String>> {
    let shell = load_public_shell(&state, &session, "/docs", "").await?;
    let books = list_public_kb_books(&state.pool, shell.logged_in).await?;
    render(DocsHomeTemplate {
        settings: shell.settings,
        logged_in: shell.logged_in,
        nav_items: shell.nav_items,
        search_query: shell.search_query,
        books,
        can_manage: shell.logged_in,
    })
}

pub async fn docs_book(
    State(state): State<AppState>,
    session: Session,
    Path(book_slug): Path<String>,
) -> AppResult<Html<String>> {
    let path = format!("/docs/{book_slug}");
    let shell = load_public_shell(&state, &session, &path, "").await?;
    let Some(book) = find_kb_book_by_slug(&state.pool, &book_slug).await? else {
        return Err(AppError::not_found("知识库不存在"));
    };
    if !shell.logged_in && !book.is_publicly_visible() {
        return Err(AppError::not_found("知识库不存在"));
    }

    let nodes = list_kb_nodes_by_book(&state.pool, book.id).await?;
    let tree = build_public_kb_tree(&nodes, shell.logged_in);
    let (doc_count, word_count) = kb_tree_stats(&tree, &nodes);
    let empty = tree.is_empty();
    let catalog = if empty {
        String::new()
    } else {
        catalog_html(&tree, &book.slug, None)
    };

    render_book_page(
        &shell.settings,
        shell.logged_in,
        shell.nav_items,
        shell.search_query,
        &book,
        &tree,
        None,
        book.title.clone(),
        if book.summary.is_empty() {
            String::new()
        } else {
            format!("<p>{}</p>", html_escape_owned(&book.summary))
        },
        Vec::new(),
        None,
        None,
        empty,
        true,
        catalog,
        doc_count,
        word_count,
    )
}

pub async fn docs_page(
    State(state): State<AppState>,
    session: Session,
    Path((book_slug, page_slug)): Path<(String, String)>,
) -> AppResult<Html<String>> {
    let path = format!("/docs/{book_slug}/{page_slug}");
    let shell = load_public_shell(&state, &session, &path, "").await?;

    let Some(book) = find_kb_book_by_slug(&state.pool, &book_slug).await? else {
        return Err(AppError::not_found("知识库不存在"));
    };
    if !shell.logged_in && !book.is_publicly_visible() {
        return Err(AppError::not_found("知识库不存在"));
    }

    let Some(page) = find_kb_node_by_book_slug(&state.pool, book.id, &page_slug).await? else {
        return Err(AppError::not_found("页面不存在"));
    };
    if !page.is_doc() {
        return Err(AppError::not_found("页面不存在"));
    }
    if !shell.logged_in && !page.is_publicly_visible() {
        return Err(AppError::not_found("页面不存在"));
    }

    let nodes = list_kb_nodes_by_book(&state.pool, book.id).await?;
    let tree = build_public_kb_tree(&nodes, shell.logged_in);
    let crumbs = kb_breadcrumbs(&nodes, &page, &book.slug);
    let (prev, next) = kb_page_neighbors(&tree, page.id);
    let content_html = render_markdown(&page.content_md);

    render_book_page(
        &shell.settings,
        shell.logged_in,
        shell.nav_items,
        shell.search_query,
        &book,
        &tree,
        Some(page.id),
        page.title.clone(),
        content_html,
        crumbs,
        prev.map(|(t, s)| KbNeighbor {
            title: t,
            href: format!("/docs/{}/{s}", book.slug),
        }),
        next.map(|(t, s)| KbNeighbor {
            title: t,
            href: format!("/docs/{}/{s}", book.slug),
        }),
        false,
        false,
        String::new(),
        0,
        0,
    )
}

fn html_escape_owned(s: &str) -> String {
    let mut out = String::new();
    html_escape(&mut out, s);
    out
}

fn render_book_page(
    settings: &crate::models::SiteSettings,
    logged_in: bool,
    nav_items: Vec<crate::models::NavItemView>,
    search_query: String,
    book: &KbBook,
    tree: &[KbTreeNode],
    active_id: Option<i64>,
    title: String,
    content_html: String,
    crumbs: Vec<crate::models::KbCrumb>,
    prev: Option<KbNeighbor>,
    next: Option<KbNeighbor>,
    empty: bool,
    is_home: bool,
    catalog_html: String,
    doc_count: usize,
    word_count: usize,
) -> AppResult<Html<String>> {
    render(DocsBookTemplate {
        settings: settings.clone(),
        logged_in,
        nav_items,
        search_query,
        book: book.clone(),
        tree_html: tree_html(tree, active_id, book.id, &book.slug, false),
        title,
        content_html,
        crumbs,
        prev,
        next,
        empty,
        is_home,
        catalog_html,
        doc_count,
        word_count,
        edit_href: if logged_in {
            Some(match active_id {
                Some(id) => format!("/admin/docs/{}?id={id}", book.id),
                None => format!("/admin/docs/{}", book.id),
            })
        } else {
            None
        },
    })
}
