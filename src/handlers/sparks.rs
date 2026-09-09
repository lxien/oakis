use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::{Html, Redirect};
use chrono::Utc;
use serde::Deserialize;
use tower_sessions::Session;

use crate::infra::error::{AppError, AppResult, render, see_other};
use crate::infra::flash::{flash_err, flash_ok};
use crate::infra::markdown::render_markdown;
use crate::infra::pagination::{ADMIN_PER_PAGE, PUBLIC_PER_PAGE, PageQuery, Pagination};
use crate::infra::state::AppState;
use crate::models::Spark;
use crate::store::{
    count_all_sparks, count_public_sparks, create_post, create_spark, delete_spark,
    find_spark_by_id, list_all_sparks, list_public_sparks, update_spark,
};
use crate::views::{
    AdminSparkEditTemplate, AdminSparksTemplate, SparkCaptureTemplate, SparksTemplate,
};
use crate::web::authz::{cloak_auth, require_user};
use crate::web::forms::{BatchIdsForm, parse_ids};
use crate::web::load_public_shell;

fn visibility_value(v: &str) -> &'static str {
    if v == Spark::VISIBILITY_PUBLIC {
        Spark::VISIBILITY_PUBLIC
    } else {
        Spark::VISIBILITY_PRIVATE
    }
}

#[derive(Debug, Deserialize)]
pub struct SparkForm {
    #[serde(default)]
    pub content_md: String,
    #[serde(default)]
    pub visibility: String,
}

#[derive(Debug, Deserialize)]
pub struct CaptureQuery {
    pub saved: Option<String>,
}

pub async fn sparks_list(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PageQuery>,
) -> AppResult<Html<String>> {
    let shell = load_public_shell(&state, &session, "/sparks", "").await?;
    let total = count_public_sparks(&state.pool).await?;
    let pagination = Pagination::new(query.page, PUBLIC_PER_PAGE, total, "/sparks");
    let mut items =
        list_public_sparks(&state.pool, pagination.limit(), pagination.offset()).await?;
    for item in &mut items {
        item.content_html = render_markdown(&item.content_md);
    }

    render(SparksTemplate {
        settings: shell.settings,
        items,
        pagination,
        logged_in: shell.logged_in,
        nav_items: shell.nav_items,
        search_query: shell.search_query,
    })
}

pub async fn spark_capture_page(
    State(state): State<AppState>,
    session: Session,
    Query(q): Query<CaptureQuery>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (_user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    Ok(Err(render(SparkCaptureTemplate {
        settings,
        error: None,
        content_md: String::new(),
        visibility: Spark::VISIBILITY_PRIVATE.to_string(),
        saved: q.saved.as_deref() == Some("1"),
    })?))
}

pub async fn spark_create(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<SparkForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (_user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let content_md = form.content_md.trim().to_string();
    let visibility = visibility_value(&form.visibility);

    if content_md.is_empty() {
        return Ok(Err(render(SparkCaptureTemplate {
            settings,
            error: Some("写点什么再保存".into()),
            content_md: String::new(),
            visibility: visibility.to_string(),
            saved: false,
        })?));
    }

    if content_md.chars().count() > 5000 {
        return Ok(Err(render(SparkCaptureTemplate {
            settings,
            error: Some("太长了，请控制在 5000 字以内".into()),
            content_md,
            visibility: visibility.to_string(),
            saved: false,
        })?));
    }

    let content_html = render_markdown(&content_md);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    create_spark(&state.pool, &content_md, &content_html, visibility, &now).await?;
    Ok(Ok(see_other("/spark?saved=1")))
}

pub async fn admin_sparks_list(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PageQuery>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let total = count_all_sparks(&state.pool).await?;
    let pagination = Pagination::new(query.page, ADMIN_PER_PAGE, total, "/admin/sparks");
    let items = list_all_sparks(&state.pool, pagination.limit(), pagination.offset()).await?;
    Ok(Err(render(AdminSparksTemplate {
        settings,
        username: user.username,
        items,
        pagination,
    })?))
}

pub async fn admin_spark_edit_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(spark) = find_spark_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("灵感不存在"));
    };
    Ok(Err(render(AdminSparkEditTemplate {
        settings,
        username: user.username,
        id: spark.id,
        content_md: spark.content_md,
        visibility: spark.visibility,
        error: None,
    })?))
}

pub async fn admin_spark_update(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<SparkForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let content_md = form.content_md.trim().to_string();
    let visibility = visibility_value(&form.visibility);

    if content_md.is_empty() {
        return Ok(Err(render(AdminSparkEditTemplate {
            settings,
            username: user.username,
            id,
            content_md: String::new(),
            visibility: visibility.to_string(),
            error: Some("内容不能为空".into()),
        })?));
    }

    let content_html = render_markdown(&content_md);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    update_spark(
        &state.pool,
        id,
        &content_md,
        &content_html,
        visibility,
        &now,
    )
    .await?;
    flash_ok(&session, "灵感已保存").await?;
    Ok(Ok(see_other("/admin/sparks")))
}

pub async fn admin_spark_delete(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Redirect> {
    if let Err(e) = require_user(&state, &session).await {
        return Err(cloak_auth(e));
    }
    delete_spark(&state.pool, id).await?;
    flash_ok(&session, "已删除").await?;
    Ok(see_other("/admin/sparks"))
}

pub async fn admin_sparks_batch_delete(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<BatchIdsForm>,
) -> AppResult<Redirect> {
    if let Err(e) = require_user(&state, &session).await {
        return Err(cloak_auth(e));
    }
    for id in parse_ids(&form.ids) {
        delete_spark(&state.pool, id).await?;
    }
    flash_ok(&session, "批量删除完成").await?;
    Ok(see_other("/admin/sparks"))
}

pub async fn admin_spark_to_post(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<SparkForm>,
) -> AppResult<Redirect> {
    if let Err(e) = require_user(&state, &session).await {
        return Err(cloak_auth(e));
    }
    let Some(spark) = find_spark_by_id(&state.pool, id).await? else {
        flash_err(&session, "灵感不存在").await?;
        return Ok(see_other("/admin/sparks"));
    };

    let content_md = {
        let trimmed = form.content_md.trim();
        if trimmed.is_empty() {
            spark.content_md.clone()
        } else {
            trimmed.to_string()
        }
    };
    if content_md.is_empty() {
        flash_err(&session, "内容不能为空").await?;
        return Ok(see_other(&format!("/admin/sparks/{id}/edit")));
    }
    if content_md.chars().count() > 5000 {
        flash_err(&session, "内容过长").await?;
        return Ok(see_other(&format!("/admin/sparks/{id}/edit")));
    }

    let visibility = if form.visibility.trim().is_empty() {
        spark.visibility.as_str()
    } else {
        visibility_value(&form.visibility)
    };
    let content_html = render_markdown(&content_md);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    update_spark(
        &state.pool,
        id,
        &content_md,
        &content_html,
        visibility,
        &now,
    )
    .await?;

    let mut titled = spark;
    titled.content_md = content_md.clone();
    let title = titled.default_title();
    let slug = format!("spark-{}-{}", id, &uuid::Uuid::new_v4().to_string()[..8]);

    let post_id = create_post(
        &state.pool,
        &title,
        &slug,
        "",
        &content_md,
        &content_html,
        "draft",
        "private",
        None,
        None,
        &now,
        &now,
    )
    .await?;
    flash_ok(&session, "已升格为草稿文章（灵感仍保留），可继续编辑").await?;
    Ok(see_other(&format!("/admin/posts/{post_id}/edit")))
}
