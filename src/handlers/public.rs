use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;
use tower_sessions::Session;

use crate::infra::error::{AppError, AppResult, render};
use crate::infra::markdown::{render_markdown, render_markdown_with_toc};
use crate::infra::pagination::{PUBLIC_PER_PAGE, PUBLIC_TAX_PER_PAGE, PageQuery, Pagination};
use crate::infra::state::AppState;
use crate::models::{Taxonomy, take_aside_list, take_aside_tax};
use crate::store::{
    count_published_posts, count_published_posts_by_taxonomy, count_search_hits,
    count_uncategorized_published_posts, find_page_by_slug, find_post_by_slug,
    find_taxonomy_by_slug, list_categories_with_counts, list_public_kb_books, list_published_posts,
    list_published_posts_by_taxonomy, list_tags_with_counts, list_uncategorized_published_posts,
    resolve_related_posts, search_hits, to_post_view, to_post_views,
};
use crate::views::{
    HomeTemplate, PageTemplate, PostTemplate, PostsIndexTemplate, SearchTemplate,
    TagsIndexTemplate, TaxonomyPageTemplate,
};
use crate::web::load_public_shell;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub page: Option<u32>,
}

pub async fn home(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PageQuery>,
) -> AppResult<Html<String>> {
    let shell = load_public_shell(&state, &session, "/", "").await?;
    let total = count_published_posts(&state.pool, shell.logged_in).await?;
    let pagination = Pagination::new(query.page, PUBLIC_PER_PAGE, total, "/");
    let posts = list_published_posts(
        &state.pool,
        shell.logged_in,
        pagination.limit(),
        pagination.offset(),
    )
    .await?;
    let mut posts = to_post_views(&state.pool, posts).await?;

    let featured = if pagination.page == 1 && !posts.is_empty() {
        Some(posts.remove(0))
    } else {
        None
    };

    let (categories, categories_more) =
        take_aside_tax(list_categories_with_counts(&state.pool, shell.logged_in).await?);
    let (tags, tags_more) =
        take_aside_tax(list_tags_with_counts(&state.pool, shell.logged_in).await?);
    let (books, books_more) =
        take_aside_list(list_public_kb_books(&state.pool, shell.logged_in).await?);

    render(HomeTemplate {
        settings: shell.settings,
        posts,
        featured,
        categories,
        categories_more,
        tags,
        tags_more,
        books,
        books_more,
        pagination,
        logged_in: shell.logged_in,
        nav_items: shell.nav_items,
        search_query: shell.search_query,
    })
}

#[derive(Deserialize)]
pub struct PostsIndexQuery {
    pub c: Option<String>,
    pub page: Option<u32>,
}

pub async fn posts_index(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PostsIndexQuery>,
) -> AppResult<Html<String>> {
    let shell = load_public_shell(&state, &session, "/posts", "").await?;
    let categories = list_categories_with_counts(&state.pool, shell.logged_in).await?;

    let active_filter = query
        .c
        .unwrap_or_default()
        .trim()
        .chars()
        .take(80)
        .collect::<String>();

    let base_url = if active_filter.is_empty() {
        "/posts".to_string()
    } else {
        format!("/posts?c={}", urlencoding_minimal(&active_filter))
    };

    let (total, raw_posts) = if active_filter.is_empty() {
        let total = count_published_posts(&state.pool, shell.logged_in).await?;
        let pagination = Pagination::new(query.page, PUBLIC_PER_PAGE, total, &base_url);
        let posts = list_published_posts(
            &state.pool,
            shell.logged_in,
            pagination.limit(),
            pagination.offset(),
        )
        .await?;
        (total, posts)
    } else if active_filter == "-" {
        let total = count_uncategorized_published_posts(&state.pool, shell.logged_in).await?;
        let pagination = Pagination::new(query.page, PUBLIC_PER_PAGE, total, &base_url);
        let posts = list_uncategorized_published_posts(
            &state.pool,
            shell.logged_in,
            pagination.limit(),
            pagination.offset(),
        )
        .await?;
        (total, posts)
    } else {
        let Some(term) = find_taxonomy_by_slug(
            &state.pool,
            Taxonomy::SCOPE_POST,
            Taxonomy::KIND_CATEGORY,
            &active_filter,
        )
        .await?
        else {
            return Err(AppError::not_found("分类不存在"));
        };
        let total =
            count_published_posts_by_taxonomy(&state.pool, term.id, shell.logged_in).await?;
        let pagination = Pagination::new(query.page, PUBLIC_PER_PAGE, total, &base_url);
        let posts = list_published_posts_by_taxonomy(
            &state.pool,
            term.id,
            shell.logged_in,
            pagination.limit(),
            pagination.offset(),
        )
        .await?;
        (total, posts)
    };

    let pagination = Pagination::new(query.page, PUBLIC_PER_PAGE, total, base_url);
    let posts = to_post_views(&state.pool, raw_posts).await?;
    let uncategorized_count =
        count_uncategorized_published_posts(&state.pool, shell.logged_in).await?;

    render(PostsIndexTemplate {
        settings: shell.settings,
        posts,
        categories,
        uncategorized_count,
        active_filter,
        pagination,
        logged_in: shell.logged_in,
        nav_items: shell.nav_items,
        search_query: shell.search_query,
    })
}

pub async fn feed_rss(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
) -> AppResult<Response> {
    let shell = load_public_shell(&state, &session, "/feed.xml", "").await?;
    let posts = list_published_posts(&state.pool, false, 30, 0).await?;
    let origin = public_origin(&state, &headers);

    let mut items = String::new();
    for post in &posts {
        let link = format!("{origin}/posts/{}", post.slug);
        let title = xml_escape(&post.title);
        let desc = xml_escape(if post.summary.is_empty() {
            post.title.as_str()
        } else {
            post.summary.as_str()
        });
        let pub_date = rfc822_date(post.display_date());
        items.push_str(&format!(
            "<item><title>{title}</title><link>{link}</link><guid>{link}</guid><description>{desc}</description><pubDate>{pub_date}</pubDate></item>"
        ));
    }

    let channel_title = xml_escape(&shell.settings.site_title);
    let channel_desc = xml_escape(if shell.settings.site_tagline.is_empty() {
        shell.settings.seo_description.as_str()
    } else {
        shell.settings.site_tagline.as_str()
    });
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><rss version="2.0"><channel><title>{channel_title}</title><link>{origin}/</link><description>{channel_desc}</description>{items}</channel></rss>"#
    );

    let mut res = body.into_response();
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/rss+xml; charset=utf-8"),
    );
    Ok(res)
}

fn public_origin(state: &AppState, headers: &HeaderMap) -> String {
    if let Some(origin) = state.config.public_origin.as_deref() {
        return origin.to_string();
    }
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    let proto = if state.config.trust_proxy {
        headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("http")
    } else {
        "http"
    };
    format!("{proto}://{host}")
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn rfc822_date(raw: &str) -> String {
    if raw.len() >= 10 {
        let y = &raw[0..4];
        let m = &raw[5..7];
        let d = &raw[8..10];
        format!(
            "{d} {month} {y} 00:00:00 +0000",
            month = month_abbr(m),
            y = y,
            d = d
        )
    } else {
        raw.to_string()
    }
}

fn month_abbr(m: &str) -> &'static str {
    match m {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => "Jan",
    }
}

pub async fn post_detail(
    State(state): State<AppState>,
    session: Session,
    Path(slug): Path<String>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let path = format!("/posts/{slug}");
    let shell = load_public_shell(&state, &session, &path, "").await?;

    let Some(post) = find_post_by_slug(&state.pool, &slug).await? else {
        return Err(AppError::not_found("文章不存在"));
    };

    if !shell.logged_in && (!post.is_published() || post.is_private()) {
        return Err(AppError::not_found("文章不存在"));
    }

    let mut post = post;
    let (html, toc) = render_markdown_with_toc(&post.content_md);
    post.content_html = html;
    let post = to_post_view(&state.pool, post).await?;
    let related = resolve_related_posts(
        &state.pool,
        post.id(),
        shell.logged_in,
        crate::models::RelatedPostRef::TARGET,
    )
    .await?;

    let (categories, categories_more) =
        take_aside_tax(list_categories_with_counts(&state.pool, shell.logged_in).await?);
    let (tags, tags_more) =
        take_aside_tax(list_tags_with_counts(&state.pool, shell.logged_in).await?);
    let (books, books_more) =
        take_aside_list(list_public_kb_books(&state.pool, shell.logged_in).await?);

    Ok(Err(render(PostTemplate {
        settings: shell.settings,
        post,
        related,
        toc,
        categories,
        categories_more,
        tags,
        tags_more,
        books,
        books_more,
        logged_in: shell.logged_in,
        nav_items: shell.nav_items,
        search_query: shell.search_query,
    })?))
}

pub async fn page_detail(
    State(state): State<AppState>,
    session: Session,
    Path(slug): Path<String>,
) -> AppResult<Html<String>> {
    let path = format!("/p/{slug}");
    let shell = load_public_shell(&state, &session, &path, "").await?;
    let Some(page) = find_page_by_slug(&state.pool, &slug).await? else {
        return Err(AppError::not_found("页面不存在"));
    };
    if !shell.logged_in && (!page.is_published() || page.is_private()) {
        return Err(AppError::not_found("页面不存在"));
    }

    let content_html = render_markdown(&page.content_md);
    render(PageTemplate {
        settings: shell.settings,
        page,
        content_html,
        logged_in: shell.logged_in,
        nav_items: shell.nav_items,
        search_query: shell.search_query,
    })
}

pub async fn search(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<SearchQuery>,
) -> AppResult<Html<String>> {
    let q = query
        .q
        .unwrap_or_default()
        .trim()
        .chars()
        .take(80)
        .collect::<String>();
    let shell = load_public_shell(&state, &session, "/search", q.clone()).await?;

    let (hits, pagination) = if q.is_empty() {
        (
            Vec::new(),
            Pagination::new(Some(1), PUBLIC_PER_PAGE, 0, "/search"),
        )
    } else {
        let total = count_search_hits(&state.pool, &q, shell.logged_in).await?;
        let base = format!("/search?q={}", urlencoding_minimal(&q));
        let pagination = Pagination::new(query.page, PUBLIC_PER_PAGE, total, base);
        let hits = search_hits(
            &state.pool,
            &q,
            shell.logged_in,
            pagination.limit(),
            pagination.offset(),
        )
        .await?;
        (hits, pagination)
    };

    render(SearchTemplate {
        settings: shell.settings,
        query: q,
        hits,
        pagination,
        logged_in: shell.logged_in,
        nav_items: shell.nav_items,
        search_query: shell.search_query,
    })
}

fn urlencoding_minimal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

pub async fn category_page(
    State(state): State<AppState>,
    session: Session,
    Path(slug): Path<String>,
    Query(query): Query<PageQuery>,
) -> AppResult<Html<String>> {
    taxonomy_list_page(
        state,
        session,
        Taxonomy::KIND_CATEGORY,
        "分类",
        slug,
        query.page,
    )
    .await
}

pub async fn tag_page(
    State(state): State<AppState>,
    session: Session,
    Path(slug): Path<String>,
    Query(query): Query<PageQuery>,
) -> AppResult<Html<String>> {
    taxonomy_list_page(state, session, Taxonomy::KIND_TAG, "标签", slug, query.page).await
}

pub async fn tags_index(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Html<String>> {
    let shell = load_public_shell(&state, &session, "/tags", "").await?;
    let tags = list_tags_with_counts(&state.pool, shell.logged_in).await?;
    render(TagsIndexTemplate {
        settings: shell.settings,
        tags,
        logged_in: shell.logged_in,
        nav_items: shell.nav_items,
        search_query: shell.search_query,
    })
}

async fn taxonomy_list_page(
    state: AppState,
    session: Session,
    kind: &str,
    kind_label: &str,
    slug: String,
    page: Option<u32>,
) -> AppResult<Html<String>> {
    let path_prefix = if kind == Taxonomy::KIND_TAG {
        "/tags"
    } else {
        "/categories"
    };
    let path = format!("{path_prefix}/{slug}");
    let shell = load_public_shell(&state, &session, &path, "").await?;
    let Some(term) = find_taxonomy_by_slug(&state.pool, Taxonomy::SCOPE_POST, kind, &slug).await?
    else {
        return Err(AppError::not_found(format!("{kind_label}不存在")));
    };
    let total = count_published_posts_by_taxonomy(&state.pool, term.id, shell.logged_in).await?;
    let base = format!("{path_prefix}/{}", term.slug);
    let pagination = Pagination::new(page, PUBLIC_TAX_PER_PAGE, total, base);
    let posts = list_published_posts_by_taxonomy(
        &state.pool,
        term.id,
        shell.logged_in,
        pagination.limit(),
        pagination.offset(),
    )
    .await?;
    let posts = to_post_views(&state.pool, posts).await?;
    render(TaxonomyPageTemplate {
        settings: shell.settings,
        term,
        kind_label: kind_label.into(),
        posts,
        pagination,
        logged_in: shell.logged_in,
        nav_items: shell.nav_items,
        search_query: shell.search_query,
    })
}
