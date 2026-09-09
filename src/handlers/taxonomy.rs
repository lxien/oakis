use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::{Html, Redirect};
use serde::Deserialize;
use tower_sessions::Session;

use crate::infra::error::{AppError, AppResult, render, see_other};
use crate::infra::flash::flash_ok;
use crate::infra::markdown::slugify;
use crate::infra::pagination::{ADMIN_PER_PAGE, Pagination};
use crate::infra::state::AppState;
use crate::models::Taxonomy;
use crate::store::{
    count_taxonomies, create_taxonomy, delete_taxonomy, find_taxonomy_by_id, list_taxonomies_page,
    update_taxonomy,
};
use crate::views::{AdminTaxonomiesTemplate, AdminTaxonomyEditTemplate};
use crate::web::authz::{cloak_auth, require_user};
use crate::web::forms::{BatchTaxDeleteForm, parse_ids};

#[derive(Deserialize)]
pub struct TaxQuery {
    pub kind: Option<String>,
    pub page: Option<u32>,
}

#[derive(Deserialize)]
pub struct CreateTaxForm {
    pub name: String,
    pub slug: String,
    pub kind: String,
}

#[derive(Deserialize)]
pub struct UpdateTaxForm {
    pub name: String,
    pub slug: String,
    pub kind: String,
}

fn normalize_kind(kind: &str) -> &'static str {
    if kind == Taxonomy::KIND_TAG {
        Taxonomy::KIND_TAG
    } else {
        Taxonomy::KIND_CATEGORY
    }
}

fn kind_label(kind: &str) -> &'static str {
    if kind == Taxonomy::KIND_TAG {
        "标签"
    } else {
        "分类"
    }
}

async fn render_tax_page(
    state: &AppState,
    settings: crate::models::SiteSettings,
    username: String,
    kind: &str,
    page: Option<u32>,
    error: Option<String>,
) -> AppResult<Html<String>> {
    let total = count_taxonomies(&state.pool, Taxonomy::SCOPE_POST, kind).await?;
    let base = format!("/admin/taxonomies?kind={kind}");
    let pagination = Pagination::new(page, ADMIN_PER_PAGE, total, base);
    let items = list_taxonomies_page(
        &state.pool,
        Taxonomy::SCOPE_POST,
        kind,
        pagination.limit(),
        pagination.offset(),
    )
    .await?;
    render(AdminTaxonomiesTemplate {
        settings,
        username,
        kind: kind.to_string(),
        kind_label: kind_label(kind).into(),
        items,
        pagination,
        error,
    })
}

fn render_edit(
    settings: crate::models::SiteSettings,
    username: String,
    id: i64,
    kind: &str,
    name: String,
    slug: String,
    error: Option<String>,
) -> AppResult<Html<String>> {
    render(AdminTaxonomyEditTemplate {
        settings,
        username,
        id,
        kind: kind.to_string(),
        kind_label: kind_label(kind).into(),
        name,
        slug,
        error,
    })
}

pub async fn taxonomies_page(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<TaxQuery>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let kind = normalize_kind(query.kind.as_deref().unwrap_or(Taxonomy::KIND_CATEGORY));
    Ok(Err(render_tax_page(
        &state,
        settings,
        user.username,
        kind,
        query.page,
        None,
    )
    .await?))
}

pub async fn taxonomy_create(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<CreateTaxForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let kind = normalize_kind(&form.kind);
    let name = form.name.trim().chars().take(40).collect::<String>();
    let mut slug = form.slug.trim().to_string();
    if slug.is_empty() {
        slug = slugify(&name);
    } else {
        slug = slugify(&slug);
    }

    if name.is_empty() {
        return Ok(Err(render_tax_page(
            &state,
            settings,
            user.username,
            kind,
            Some(1),
            Some("名称不能为空".into()),
        )
        .await?));
    }

    if let Err(e) = create_taxonomy(&state.pool, Taxonomy::SCOPE_POST, kind, &name, &slug).await {
        tracing::warn!(error = %e, "taxonomy create failed");
        return Ok(Err(render_tax_page(
            &state,
            settings,
            user.username,
            kind,
            Some(1),
            Some("保存失败，名称或 slug 可能重复".into()),
        )
        .await?));
    }

    flash_ok(&session, "已保存").await?;
    Ok(Ok(see_other(&format!("/admin/taxonomies?kind={kind}"))))
}

pub async fn taxonomy_edit_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Query(query): Query<TaxQuery>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let Some(item) = find_taxonomy_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("条目不存在"));
    };
    let kind = normalize_kind(query.kind.as_deref().unwrap_or(&item.kind));
    Ok(Err(render_edit(
        settings,
        user.username,
        item.id,
        kind,
        item.name,
        item.slug,
        None,
    )?))
}

pub async fn taxonomy_update(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<UpdateTaxForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let kind = normalize_kind(&form.kind);
    let Some(_) = find_taxonomy_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("条目不存在"));
    };

    let name = form.name.trim().chars().take(40).collect::<String>();
    let mut slug = form.slug.trim().to_string();
    if slug.is_empty() {
        slug = slugify(&name);
    } else {
        slug = slugify(&slug);
    }

    if name.is_empty() {
        return Ok(Err(render_edit(
            settings,
            user.username,
            id,
            kind,
            name,
            slug,
            Some("名称不能为空".into()),
        )?));
    }

    if let Err(e) = update_taxonomy(&state.pool, id, &name, &slug).await {
        tracing::warn!(error = %e, taxonomy_id = id, "taxonomy update failed");
        return Ok(Err(render_edit(
            settings,
            user.username,
            id,
            kind,
            name,
            slug,
            Some("更新失败，名称或 slug 可能重复".into()),
        )?));
    }

    flash_ok(&session, "已保存").await?;
    Ok(Ok(see_other(&format!("/admin/taxonomies?kind={kind}"))))
}

pub async fn taxonomy_delete(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Query(query): Query<TaxQuery>,
) -> AppResult<Redirect> {
    if let Err(e) = require_user(&state, &session).await {
        return Err(cloak_auth(e));
    }
    delete_taxonomy(&state.pool, id).await?;
    let kind = normalize_kind(query.kind.as_deref().unwrap_or(Taxonomy::KIND_CATEGORY));
    flash_ok(&session, "已删除").await?;
    Ok(see_other(&format!("/admin/taxonomies?kind={kind}")))
}

pub async fn taxonomies_batch_delete(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<BatchTaxDeleteForm>,
) -> AppResult<Redirect> {
    if let Err(e) = require_user(&state, &session).await {
        return Err(cloak_auth(e));
    }
    let kind = normalize_kind(&form.kind);
    for id in parse_ids(&form.ids) {
        delete_taxonomy(&state.pool, id).await?;
    }
    flash_ok(&session, "批量删除完成").await?;
    Ok(see_other(&format!("/admin/taxonomies?kind={kind}")))
}
