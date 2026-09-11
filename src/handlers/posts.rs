use std::collections::HashSet;

use axum::Form;
use axum::Json;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use chrono::Utc;
use serde::Serialize;
use tower_sessions::Session;

use crate::infra::error::{AppError, AppResult, render, see_other};
use crate::infra::flash::{flash_err, flash_ok};
use crate::infra::markdown::{render_markdown, slugify};
use crate::infra::pagination::{ADMIN_PER_PAGE, PageQuery, Pagination};
use crate::infra::state::AppState;
use crate::infra::timefmt;
use crate::infra::upload::{
    delete_media_file, delete_post_image, sanitize_cover_url, save_media, save_post_image,
};
use crate::models::{RelatedPostRef, SiteSettings, Taxonomy};
use crate::store::{
    count_admin_posts_search, count_all_posts, create_post, delete_post, find_post_by_id,
    insert_media, list_admin_posts_search, list_all_posts, list_related_ids,
    list_related_refs_ordered, list_taxonomies, list_taxonomies_for_post, post_slug_taken,
    set_post_relations, set_post_taxonomies, update_post,
};
use crate::views::{AdminPostEditTemplate, AdminPostsTemplate};
use crate::web::TaxOption;
use crate::web::authz::{cloak_auth, require_user};
use crate::web::forms::{BatchIdsForm, parse_ids};

pub async fn posts_list(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PageQuery>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let total = count_all_posts(&state.pool).await?;
    let pagination = Pagination::new(query.page, ADMIN_PER_PAGE, total, "/admin/posts");
    let posts = list_all_posts(&state.pool, pagination.limit(), pagination.offset()).await?;
    Ok(Err(render(AdminPostsTemplate {
        settings,
        username: user.username,
        posts,
        pagination,
    })?))
}

async fn edit_view(
    state: &AppState,
    settings: SiteSettings,
    username: String,
    is_new: bool,
    content_id: i64,
    form_action: String,
    title: String,
    slug: String,
    summary: String,
    content_md: String,
    cover_url: String,
    status: String,
    visibility: String,
    published_at_local: String,
    selected: &HashSet<i64>,
    related: Vec<RelatedPostRef>,
    error: Option<String>,
) -> AppResult<Html<String>> {
    let cats = list_taxonomies(&state.pool, Taxonomy::SCOPE_POST, Taxonomy::KIND_CATEGORY).await?;
    let tags = list_taxonomies(&state.pool, Taxonomy::SCOPE_POST, Taxonomy::KIND_TAG).await?;
    render(AdminPostEditTemplate {
        settings,
        username,
        is_new,
        content_id,
        form_action,
        title,
        slug,
        summary,
        content_md,
        cover_url,
        status,
        visibility,
        published_at_local,
        categories: TaxOption::from_list(cats, selected),
        tags: TaxOption::from_list(tags, selected),
        related,
        related_max: RelatedPostRef::MANUAL_MAX,
        error,
    })
}

async fn related_refs_for_ids(state: &AppState, ids: &[i64]) -> AppResult<Vec<RelatedPostRef>> {
    list_related_refs_ordered(&state.pool, ids).await
}

pub async fn post_new_page(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    Ok(Err(edit_view(
        &state,
        settings,
        user.username,
        true,
        0,
        "/admin/posts/new".into(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        "published".into(),
        "public".into(),
        timefmt::now_local_input(),
        &HashSet::new(),
        Vec::new(),
        None,
    )
    .await?))
}

struct ParsedPostForm {
    title: String,
    slug: String,
    summary: String,
    content_md: String,
    status: String,
    visibility: String,
    published_at: String,
    remove_cover: bool,
    cover_url: String,
    taxonomy_ids: Vec<i64>,
    related_ids: Vec<i64>,
}

async fn parse_post_multipart(multipart: &mut Multipart) -> AppResult<ParsedPostForm> {
    let mut title = String::new();
    let mut slug = String::new();
    let mut summary = String::new();
    let mut content_md = String::new();
    let mut status = "draft".into();
    let mut visibility = "public".into();
    let mut published_at = String::new();
    let mut remove_cover = false;
    let mut cover_url = String::new();
    let mut taxonomy_ids = Vec::new();
    let mut related_ids = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::bad_request("表单数据无效或过大，请减少附件后重试"))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "title" | "slug" | "summary" | "content_md" | "status" | "visibility" | "cover_url"
            | "published_at" => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| AppError::bad_request("读取表单字段失败，请重试"))?;
                match name.as_str() {
                    "title" => title = text.trim().chars().take(200).collect(),
                    "slug" => slug = text.trim().chars().take(200).collect(),
                    "summary" => summary = text.trim().chars().take(300).collect(),
                    "content_md" => content_md = text,
                    "status" => status = text,
                    "visibility" => visibility = text,
                    "cover_url" => cover_url = text.trim().to_string(),
                    "published_at" => published_at = text.trim().to_string(),
                    _ => {}
                }
            }
            "remove_cover" => {
                let v = field.text().await.unwrap_or_default();
                remove_cover = v == "1" || v.eq_ignore_ascii_case("on");
            }
            "category_ids" | "tag_ids" => {
                let text = field.text().await.unwrap_or_default();
                if let Ok(id) = text.trim().parse::<i64>() {
                    taxonomy_ids.push(id);
                }
            }
            "related_ids" => {
                let text = field.text().await.unwrap_or_default();
                if let Ok(id) = text.trim().parse::<i64>() {
                    if !related_ids.contains(&id) {
                        related_ids.push(id);
                    }
                }
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    taxonomy_ids.sort_unstable();
    taxonomy_ids.dedup();
    if related_ids.len() > RelatedPostRef::MANUAL_MAX {
        related_ids.truncate(RelatedPostRef::MANUAL_MAX);
    }

    Ok(ParsedPostForm {
        title,
        slug,
        summary,
        content_md,
        status,
        visibility,
        published_at,
        remove_cover,
        cover_url,
        taxonomy_ids,
        related_ids,
    })
}

fn normalize_slug(title: &str, slug: &str) -> String {
    let slug = slug.trim();
    if slug.is_empty() {
        slugify(title)
    } else {
        slugify(slug)
    }
}

fn status_value(status: &str) -> &'static str {
    if status == "published" {
        "published"
    } else {
        "draft"
    }
}

fn visibility_value(visibility: &str) -> &'static str {
    if visibility == "private" {
        "private"
    } else {
        "public"
    }
}

pub async fn post_create(
    State(state): State<AppState>,
    session: Session,
    mut multipart: Multipart,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let form = match parse_post_multipart(&mut multipart).await {
        Ok(f) => f,
        Err(e) => {
            flash_err(&session, e.user_message()).await?;
            return Ok(Ok(see_other("/admin/posts/new")));
        }
    };
    let selected: HashSet<i64> = form.taxonomy_ids.iter().copied().collect();
    let related_ids = form.related_ids.clone();
    let related = related_refs_for_ids(&state, &related_ids).await?;
    let title = form.title;
    let slug = normalize_slug(&title, &form.slug);
    let summary = form.summary;
    let content_md = form.content_md;
    let status = status_value(&form.status);
    let visibility = visibility_value(&form.visibility);
    let content_html = render_markdown(&content_md);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let published_at = timefmt::resolve_published_input(&form.published_at);
    let published_at_local = timefmt::form_local_or(&published_at, &form.published_at);

    let cover_url = if form.remove_cover {
        String::new()
    } else {
        sanitize_cover_url(&form.cover_url)
    };

    if title.is_empty() {
        return Ok(Err(edit_view(
            &state,
            settings,
            user.username,
            true,
            0,
            "/admin/posts/new".into(),
            title,
            slug,
            summary,
            content_md,
            cover_url,
            status.into(),
            visibility.into(),
            published_at_local,
            &selected,
            related,
            Some("标题不能为空".into()),
        )
        .await?));
    }

    if post_slug_taken(&state.pool, &slug, None).await? {
        return Ok(Err(edit_view(
            &state,
            settings,
            user.username,
            true,
            0,
            "/admin/posts/new".into(),
            title,
            slug,
            summary,
            content_md,
            cover_url,
            status.into(),
            visibility.into(),
            published_at_local,
            &selected,
            related,
            Some("Slug 已被占用".into()),
        )
        .await?));
    }

    let cover_db = if cover_url.is_empty() {
        None
    } else {
        Some(cover_url.as_str())
    };

    let result = create_post(
        &state.pool,
        &title,
        &slug,
        &summary,
        &content_md,
        &content_html,
        status,
        visibility,
        cover_db,
        Some(&published_at),
        &now,
        &now,
    )
    .await;

    match result {
        Ok(post_id) => {
            set_post_taxonomies(&state.pool, post_id, &form.taxonomy_ids).await?;
            set_post_relations(&state.pool, post_id, &related_ids).await?;
            flash_ok(&session, "文章已保存").await?;
            Ok(Ok(see_other("/admin/posts")))
        }
        Err(e) => {
            tracing::warn!(error = %e, "post create failed");
            Ok(Err(edit_view(
                &state,
                settings,
                user.username,
                true,
                0,
                "/admin/posts/new".into(),
                title,
                slug,
                summary,
                content_md,
                cover_url,
                status.into(),
                visibility.into(),
                published_at_local,
                &selected,
                related,
                Some("保存失败，请稍后重试".into()),
            )
            .await?))
        }
    }
}

pub async fn post_edit_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(post) = find_post_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("文章不存在"));
    };

    let selected: HashSet<i64> = list_taxonomies_for_post(&state.pool, post.id)
        .await?
        .into_iter()
        .map(|t| t.id)
        .collect();
    let cover = post.cover().to_string();
    let published_at_local = timefmt::to_local_input(post.published_at.as_deref());
    let related_ids = list_related_ids(&state.pool, post.id).await?;
    let related = related_refs_for_ids(&state, &related_ids).await?;
    Ok(Err(edit_view(
        &state,
        settings,
        user.username,
        false,
        post.id,
        format!("/admin/posts/{}/edit", post.id),
        post.title,
        post.slug,
        post.summary,
        post.content_md,
        cover,
        post.status,
        post.visibility.clone(),
        published_at_local,
        &selected,
        related,
        None,
    )
    .await?))
}

pub async fn post_update(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let Some(existing) = find_post_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("文章不存在"));
    };

    let form = match parse_post_multipart(&mut multipart).await {
        Ok(f) => f,
        Err(e) => {
            flash_err(&session, e.user_message()).await?;
            return Ok(Ok(see_other(&format!("/admin/posts/{id}/edit"))));
        }
    };
    let selected: HashSet<i64> = form.taxonomy_ids.iter().copied().collect();
    let related_ids: Vec<i64> = form
        .related_ids
        .iter()
        .copied()
        .filter(|&rid| rid != id)
        .collect();
    let related = related_refs_for_ids(&state, &related_ids).await?;
    let title = form.title;
    let slug = normalize_slug(&title, &form.slug);
    let summary = form.summary;
    let content_md = form.content_md;
    let status = status_value(&form.status);
    let visibility = visibility_value(&form.visibility);
    let content_html = render_markdown(&content_md);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let published_at = timefmt::resolve_published_input(&form.published_at);
    let published_at_local = timefmt::form_local_or(&published_at, &form.published_at);

    let mut cover_url = existing.cover().to_string();
    if form.remove_cover {
        if !cover_url.is_empty() {
            delete_post_image(&cover_url, &state.config.posts_dir()).await;
        }
        cover_url.clear();
    } else {
        let next = sanitize_cover_url(&form.cover_url);
        if next != cover_url {
            if !cover_url.is_empty() {
                delete_post_image(&cover_url, &state.config.posts_dir()).await;
            }
            cover_url = next;
        }
    }

    if title.is_empty() {
        return Ok(Err(edit_view(
            &state,
            settings,
            user.username,
            false,
            id,
            format!("/admin/posts/{id}/edit"),
            title,
            slug,
            summary,
            content_md,
            cover_url,
            status.into(),
            visibility.into(),
            published_at_local,
            &selected,
            related,
            Some("标题不能为空".into()),
        )
        .await?));
    }

    if post_slug_taken(&state.pool, &slug, Some(id)).await? {
        return Ok(Err(edit_view(
            &state,
            settings,
            user.username,
            false,
            id,
            format!("/admin/posts/{id}/edit"),
            title,
            slug,
            summary,
            content_md,
            cover_url,
            status.into(),
            visibility.into(),
            published_at_local,
            &selected,
            related,
            Some("Slug 已被占用".into()),
        )
        .await?));
    }

    let cover_db = if cover_url.is_empty() {
        None
    } else {
        Some(cover_url.as_str())
    };

    if let Err(e) = update_post(
        &state.pool,
        id,
        &title,
        &slug,
        &summary,
        &content_md,
        &content_html,
        status,
        visibility,
        cover_db,
        &published_at,
        &now,
    )
    .await
    {
        tracing::warn!(error = %e, post_id = id, "post update failed");
        return Ok(Err(edit_view(
            &state,
            settings,
            user.username,
            false,
            id,
            format!("/admin/posts/{id}/edit"),
            title,
            slug,
            summary,
            content_md,
            cover_url,
            status.into(),
            visibility.into(),
            published_at_local,
            &selected,
            related,
            Some("保存失败，请稍后重试".into()),
        )
        .await?));
    }

    set_post_taxonomies(&state.pool, id, &form.taxonomy_ids).await?;
    set_post_relations(&state.pool, id, &related_ids).await?;
    flash_ok(&session, "文章已保存").await?;
    Ok(Ok(see_other("/admin/posts")))
}

const RELATED_PICKER_LIMIT: i64 = 24;

#[derive(serde::Deserialize)]
pub struct RelatedPostsApiQuery {
    pub q: Option<String>,
    pub page: Option<u32>,
    pub exclude: Option<i64>,
}

#[derive(Serialize)]
pub struct RelatedPostsApiItem {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub date: String,
}

#[derive(Serialize)]
pub struct RelatedPostsApiResponse {
    pub page: u32,
    pub limit: u32,
    pub total: i64,
    pub items: Vec<RelatedPostsApiItem>,
}

pub async fn posts_search_api(
    State(state): State<AppState>,
    session: Session,
    Query(q): Query<RelatedPostsApiQuery>,
) -> AppResult<Json<RelatedPostsApiResponse>> {
    if require_user(&state, &session).await.is_err() {
        return Err(AppError::not_found("页面不存在"));
    }

    let page = q.page.unwrap_or(1).max(1);
    let exclude = q.exclude.filter(|&id| id > 0);
    let keyword = q.q.unwrap_or_default();
    let total = count_admin_posts_search(&state.pool, &keyword, exclude).await?;
    let offset = ((page as i64) - 1) * RELATED_PICKER_LIMIT;
    let rows =
        list_admin_posts_search(&state.pool, &keyword, exclude, RELATED_PICKER_LIMIT, offset)
            .await?;
    let items = rows
        .into_iter()
        .map(|p| {
            let date = p.short_date().to_string();
            RelatedPostsApiItem {
                id: p.id,
                title: p.title,
                slug: p.slug,
                status: p.status,
                date,
            }
        })
        .collect();

    Ok(Json(RelatedPostsApiResponse {
        page,
        limit: RELATED_PICKER_LIMIT as u32,
        total,
        items,
    }))
}

async fn delete_post_by_id(state: &AppState, id: i64) -> AppResult<()> {
    if let Some(post) = find_post_by_id(&state.pool, id).await? {
        if post.has_cover() {
            delete_post_image(post.cover(), &state.config.posts_dir()).await;
        }
    }
    delete_post(&state.pool, id).await?;
    Ok(())
}

pub async fn post_delete(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Redirect> {
    if let Err(e) = require_user(&state, &session).await {
        return Err(cloak_auth(e));
    }
    delete_post_by_id(&state, id).await?;
    flash_ok(&session, "已删除").await?;
    Ok(see_other("/admin/posts"))
}

pub async fn posts_batch_delete(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<BatchIdsForm>,
) -> AppResult<Redirect> {
    if let Err(e) = require_user(&state, &session).await {
        return Err(cloak_auth(e));
    }
    for id in parse_ids(&form.ids) {
        delete_post_by_id(&state, id).await?;
    }
    flash_ok(&session, "批量删除完成").await?;
    Ok(see_other("/admin/posts"))
}

#[derive(Serialize)]
struct UploadOk {
    url: String,
    mime: String,
    name: String,
}

#[derive(Serialize)]
struct UploadErr {
    error: String,
}

pub async fn upload_image(
    State(state): State<AppState>,
    session: Session,
    mut multipart: Multipart,
) -> Response {
    if require_user(&state, &session).await.is_err() {
        return (
            StatusCode::NOT_FOUND,
            Json(UploadErr {
                error: "not found".into(),
            }),
        )
            .into_response();
    }

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut original_name = String::from("image");
    while let Some(field) = match multipart.next_field().await {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(error = %e, "upload multipart parse failed");
            return (
                StatusCode::BAD_REQUEST,
                Json(UploadErr {
                    error: "表单解析失败".into(),
                }),
            )
                .into_response();
        }
    } {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "image" {
            if let Some(fname) = field.file_name().map(|s| s.to_string()) {
                if !fname.trim().is_empty() {
                    original_name = fname;
                }
            }
            match field.bytes().await {
                Ok(bytes) if !bytes.is_empty() => file_bytes = Some(bytes.to_vec()),
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "upload read failed");
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(UploadErr {
                            error: "读取文件失败".into(),
                        }),
                    )
                        .into_response();
                }
            }
        } else {
            let _ = field.bytes().await;
        }
    }

    let Some(bytes) = file_bytes else {
        return (
            StatusCode::BAD_REQUEST,
            Json(UploadErr {
                error: "请选择图片文件".into(),
            }),
        )
            .into_response();
    };

    match save_media(&bytes, &original_name, &state.config.media_dir()).await {
        Ok(saved) => {
            if let Err(e) = insert_media(
                &state.pool,
                &saved.name,
                &saved.url,
                &saved.mime,
                saved.size,
            )
            .await
            {
                tracing::warn!(error = %e, "upload insert_media failed");
                delete_media_file(&saved.url, &state.config.media_dir()).await;
                return (
                    StatusCode::BAD_REQUEST,
                    Json(UploadErr {
                        error: "保存失败，请稍后重试".into(),
                    }),
                )
                    .into_response();
            }
            (
                StatusCode::OK,
                Json(UploadOk {
                    url: saved.url,
                    mime: saved.mime,
                    name: saved.name,
                }),
            )
                .into_response()
        }
        Err(_) => match save_post_image(&bytes, &state.config.posts_dir()).await {
            Ok(url) => (
                StatusCode::OK,
                Json(UploadOk {
                    url,
                    mime: "image/jpeg".into(),
                    name: original_name,
                }),
            )
                .into_response(),
            Err(e) => {
                tracing::warn!(error = %e, "upload fallback failed");
                (
                    StatusCode::BAD_REQUEST,
                    Json(UploadErr {
                        error: "上传失败，请检查文件类型或大小".into(),
                    }),
                )
                    .into_response()
            }
        },
    }
}
