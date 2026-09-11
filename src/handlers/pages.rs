use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::{Html, Redirect};
use chrono::Utc;
use serde::Deserialize;
use tower_sessions::Session;

use crate::infra::error::{AppError, AppResult, render, see_other};
use crate::infra::flash::flash_ok;
use crate::infra::markdown::slugify;
use crate::infra::pagination::{ADMIN_PER_PAGE, PageQuery, Pagination};
use crate::infra::state::AppState;
use crate::store::{
    count_all_pages, create_page, delete_page, find_page_by_id, list_all_pages, page_slug_taken,
    purge_page_from_nav, update_page,
};
use crate::views::{AdminPageEditTemplate, AdminPagesTemplate};
use crate::web::authz::{cloak_auth, require_user};
use crate::web::forms::{BatchIdsForm, parse_ids};

#[derive(Debug, Deserialize)]
pub struct PageForm {
    pub title: String,
    pub slug: String,
    pub content_md: String,
    pub status: String,
    pub visibility: String,
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

fn normalize_slug(title: &str, slug: &str) -> String {
    let slug = slug.trim();
    if slug.is_empty() {
        slugify(title)
    } else {
        slugify(slug)
    }
}

fn edit_view(
    settings: crate::models::SiteSettings,
    username: String,
    is_new: bool,
    content_id: i64,
    form_action: String,
    title: String,
    slug: String,
    content_md: String,
    status: String,
    visibility: String,
    error: Option<String>,
) -> AppResult<Html<String>> {
    render(AdminPageEditTemplate {
        settings,
        username,
        is_new,
        content_id,
        form_action,
        title,
        slug,
        content_md,
        status,
        visibility,
        error,
    })
}

pub async fn pages_list(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PageQuery>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let total = count_all_pages(&state.pool).await?;
    let pagination = Pagination::new(query.page, ADMIN_PER_PAGE, total, "/admin/pages");
    let pages = list_all_pages(&state.pool, pagination.limit(), pagination.offset()).await?;
    Ok(Err(render(AdminPagesTemplate {
        settings,
        username: user.username,
        pages,
        pagination,
    })?))
}

pub async fn page_new_page(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    Ok(Err(edit_view(
        settings,
        user.username,
        true,
        0,
        "/admin/pages/new".into(),
        String::new(),
        String::new(),
        String::new(),
        "published".into(),
        "public".into(),
        None,
    )?))
}

pub async fn page_create(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<PageForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let title = form.title.trim().chars().take(120).collect::<String>();
    let slug = normalize_slug(&title, &form.slug);
    let content_md = form.content_md;
    let status = status_value(&form.status);
    let visibility = visibility_value(&form.visibility);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let fail = |msg: String| {
        edit_view(
            settings.clone(),
            user.username.clone(),
            true,
            0,
            "/admin/pages/new".into(),
            title.clone(),
            slug.clone(),
            content_md.clone(),
            status.into(),
            visibility.into(),
            Some(msg),
        )
    };

    if title.is_empty() {
        return Ok(Err(fail("标题不能为空".into())?));
    }
    if page_slug_taken(&state.pool, &slug, None).await? {
        return Ok(Err(fail("Slug 已被占用".into())?));
    }

    create_page(
        &state.pool,
        &title,
        &slug,
        &content_md,
        status,
        visibility,
        &now,
        &now,
    )
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "page save failed");
        AppError::bad_request("保存失败，请稍后重试")
    })?;

    flash_ok(&session, "页面已保存").await?;
    Ok(Ok(see_other("/admin/pages")))
}

pub async fn page_edit_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(page) = find_page_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("页面不存在"));
    };
    Ok(Err(edit_view(
        settings,
        user.username,
        false,
        page.id,
        format!("/admin/pages/{}/edit", page.id),
        page.title,
        page.slug,
        page.content_md,
        page.status,
        page.visibility,
        None,
    )?))
}

pub async fn page_update(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<PageForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(_) = find_page_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("页面不存在"));
    };

    let title = form.title.trim().chars().take(120).collect::<String>();
    let slug = normalize_slug(&title, &form.slug);
    let content_md = form.content_md;
    let status = status_value(&form.status);
    let visibility = visibility_value(&form.visibility);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let fail = |msg: String| {
        edit_view(
            settings.clone(),
            user.username.clone(),
            false,
            id,
            format!("/admin/pages/{id}/edit"),
            title.clone(),
            slug.clone(),
            content_md.clone(),
            status.into(),
            visibility.into(),
            Some(msg),
        )
    };

    if title.is_empty() {
        return Ok(Err(fail("标题不能为空".into())?));
    }
    if page_slug_taken(&state.pool, &slug, Some(id)).await? {
        return Ok(Err(fail("Slug 已被占用".into())?));
    }

    update_page(
        &state.pool,
        id,
        &title,
        &slug,
        &content_md,
        status,
        visibility,
        &now,
    )
    .await
    .map_err(|e| {
        tracing::warn!(error = %e, "page save failed");
        AppError::bad_request("保存失败，请稍后重试")
    })?;

    flash_ok(&session, "页面已保存").await?;
    Ok(Ok(see_other("/admin/pages")))
}

pub async fn page_delete(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Result<Redirect, Html<String>>> {
    match require_user(&state, &session).await {
        Ok(_) => {}
        Err(e) => return Err(cloak_auth(e)),
    };
    delete_page(&state.pool, id).await?;
    purge_page_from_nav(&state.pool, id).await?;
    flash_ok(&session, "已删除").await?;
    Ok(Ok(see_other("/admin/pages")))
}

pub async fn pages_batch_delete(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<BatchIdsForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    match require_user(&state, &session).await {
        Ok(_) => {}
        Err(e) => return Err(cloak_auth(e)),
    };
    for id in parse_ids(&form.ids) {
        delete_page(&state.pool, id).await?;
        purge_page_from_nav(&state.pool, id).await?;
    }
    flash_ok(&session, "批量删除完成").await?;
    Ok(Ok(see_other("/admin/pages")))
}
