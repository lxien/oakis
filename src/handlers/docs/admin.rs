use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::{Html, Redirect};
use chrono::Utc;
use tower_sessions::Session;

use crate::infra::error::{AppError, AppResult, render, see_other};
use crate::infra::flash::{flash_err, flash_ok};
use crate::infra::markdown::{render_markdown, slugify};
use crate::infra::state::AppState;
use crate::infra::timefmt;
use crate::infra::upload::sanitize_cover_url;
use crate::models::KbNode;
use crate::store::{
    build_kb_tree, create_kb_book, create_kb_node, delete_kb_book, delete_kb_node,
    find_kb_book_by_id, find_kb_node_by_id, kb_book_slug_taken, kb_node_slug_taken, kb_tree_stats,
    list_kb_books, list_kb_nodes_by_book, move_kb_node, rename_kb_node, update_kb_book,
    update_kb_node,
};
use crate::views::{AdminDocsBookTemplate, AdminDocsListTemplate, AdminDocsSettingsTemplate};
use crate::web::authz::{cloak_auth, require_user};

use super::{
    BookForm, CreateNodeForm, MoveNodeForm, NodeQuery, RenameNodeForm, SaveNodeForm, catalog_html,
    normalize_slug, parse_parent_id, status_value, tree_html, visibility_value,
};

pub async fn admin_docs_list(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let books = list_kb_books(&state.pool).await?;
    Ok(Err(render(AdminDocsListTemplate {
        settings,
        username: user.username,
        books,
        error: None,
    })?))
}

pub async fn admin_book_create(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<BookForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let title = form.title.trim().chars().take(80).collect::<String>();
    let slug = normalize_slug(&title, &form.slug);
    let summary = form.summary.trim().chars().take(200).collect::<String>();
    let cover_url = sanitize_cover_url(&form.cover_url);
    let status = status_value(&form.status);
    let visibility = visibility_value(&form.visibility);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    if title.is_empty() {
        let books = list_kb_books(&state.pool).await?;
        return Ok(Err(render(AdminDocsListTemplate {
            settings,
            username: user.username,
            books,
            error: Some("标题不能为空".into()),
        })?));
    }
    if kb_book_slug_taken(&state.pool, &slug, None).await? {
        let books = list_kb_books(&state.pool).await?;
        return Ok(Err(render(AdminDocsListTemplate {
            settings,
            username: user.username,
            books,
            error: Some("Slug 已被占用".into()),
        })?));
    }

    let id = create_kb_book(
        &state.pool,
        &title,
        &slug,
        &summary,
        &cover_url,
        status,
        visibility,
        &now,
    )
    .await?;
    flash_ok(&session, "知识库已创建").await?;
    Ok(Ok(see_other(&format!("/admin/docs/{id}"))))
}

pub async fn admin_book_settings_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(book) = find_kb_book_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("知识库不存在"));
    };
    Ok(Err(render(AdminDocsSettingsTemplate {
        settings,
        username: user.username,
        book,
        error: None,
    })?))
}

pub async fn admin_book_save(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<BookForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(book) = find_kb_book_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("知识库不存在"));
    };

    let title = form.title.trim().chars().take(80).collect::<String>();
    let slug = normalize_slug(&title, &form.slug);
    let summary = form.summary.trim().chars().take(200).collect::<String>();
    let cover_url = sanitize_cover_url(&form.cover_url);
    let status = status_value(&form.status);
    let visibility = visibility_value(&form.visibility);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    if title.is_empty() {
        return Ok(Err(render(AdminDocsSettingsTemplate {
            settings,
            username: user.username,
            book,
            error: Some("标题不能为空".into()),
        })?));
    }
    if kb_book_slug_taken(&state.pool, &slug, Some(id)).await? {
        return Ok(Err(render(AdminDocsSettingsTemplate {
            settings,
            username: user.username,
            book,
            error: Some("Slug 已被占用".into()),
        })?));
    }

    update_kb_book(
        &state.pool,
        id,
        &title,
        &slug,
        &summary,
        &cover_url,
        status,
        visibility,
        &now,
    )
    .await?;
    flash_ok(&session, "知识库设置已保存").await?;
    Ok(Ok(see_other(&format!("/admin/docs/books/{id}/settings"))))
}

pub async fn admin_book_delete(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Result<Redirect, Html<String>>> {
    match require_user(&state, &session).await {
        Ok(_) => {}
        Err(e) => return Err(cloak_auth(e)),
    };
    delete_kb_book(&state.pool, id).await?;
    flash_ok(&session, "知识库已删除").await?;
    Ok(Ok(see_other("/admin/docs")))
}

pub async fn admin_book_page(
    State(state): State<AppState>,
    session: Session,
    Path(book_id): Path<i64>,
    Query(query): Query<NodeQuery>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(book) = find_kb_book_by_id(&state.pool, book_id).await? else {
        return Err(AppError::not_found("知识库不存在"));
    };

    let nodes = list_kb_nodes_by_book(&state.pool, book_id).await?;
    let tree = build_kb_tree(&nodes);
    let selected = match query.id {
        Some(id) => find_kb_node_by_id(&state.pool, id).await?,
        None => None,
    };
    if let Some(ref n) = selected {
        if n.book_id != book_id {
            return Err(AppError::not_found("页面不存在"));
        }
    }

    let (
        selected_id,
        is_folder,
        title,
        slug,
        content_md,
        status,
        visibility,
        published_at_local,
        form_action,
    ) = if let Some(n) = selected.as_ref() {
        (
            Some(n.id),
            n.is_folder(),
            n.title.clone(),
            n.slug.clone(),
            n.content_md.clone(),
            n.status.clone(),
            n.visibility.clone(),
            timefmt::to_local_input(n.published_at.as_deref()),
            format!("/admin/docs/nodes/{}/save", n.id),
        )
    } else {
        (
            None,
            false,
            String::new(),
            String::new(),
            String::new(),
            "published".into(),
            "public".into(),
            timefmt::now_local_input(),
            String::new(),
        )
    };

    let tree_html = tree_html(&tree, selected_id, book_id, &book.slug, true);
    let is_home = selected_id.is_none();
    let (doc_count, word_count) = if is_home {
        kb_tree_stats(&tree, &nodes)
    } else {
        (0, 0)
    };
    let catalog_html = if is_home && !nodes.is_empty() {
        catalog_html(&tree, &book.slug, Some(book_id))
    } else {
        String::new()
    };

    Ok(Err(render(AdminDocsBookTemplate {
        settings,
        username: user.username,
        book,
        tree_html,
        has_nodes: !nodes.is_empty(),
        selected_id,
        is_folder,
        title,
        slug,
        content_md,
        status,
        visibility,
        published_at_local,
        form_action,
        catalog_html,
        doc_count,
        word_count,
        error: None,
    })?))
}

pub async fn admin_node_create(
    State(state): State<AppState>,
    session: Session,
    Path(book_id): Path<i64>,
    Form(form): Form<CreateNodeForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    match require_user(&state, &session).await {
        Ok(_) => {}
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(_) = find_kb_book_by_id(&state.pool, book_id).await? else {
        return Err(AppError::not_found("知识库不存在"));
    };

    let title = form.title.trim().chars().take(120).collect::<String>();
    if title.is_empty() {
        flash_err(&session, "标题不能为空").await?;
        return Ok(Ok(see_other(&format!("/admin/docs/{book_id}"))));
    }

    let node_type = if form.node_type == KbNode::TYPE_FOLDER {
        KbNode::TYPE_FOLDER
    } else {
        KbNode::TYPE_DOC
    };

    let parent_id = parse_parent_id(&form.parent_id);
    if let Some(pid) = parent_id {
        let Some(parent) = find_kb_node_by_id(&state.pool, pid).await? else {
            flash_err(&session, "父节点不存在").await?;
            return Ok(Ok(see_other(&format!("/admin/docs/{book_id}"))));
        };
        if parent.book_id != book_id {
            flash_err(&session, "父节点不属于该知识库").await?;
            return Ok(Ok(see_other(&format!("/admin/docs/{book_id}"))));
        }
    }

    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut slug = slugify(&title);
    if kb_node_slug_taken(&state.pool, book_id, &slug, None).await? {
        slug = format!("{slug}-{}", &uuid::Uuid::new_v4().to_string()[..6]);
    }

    let (status, visibility, content_md, content_html, published_at) =
        ("published", "public", "", "", None);

    let id = create_kb_node(
        &state.pool,
        book_id,
        parent_id,
        node_type,
        &title,
        &slug,
        content_md,
        content_html,
        status,
        visibility,
        published_at,
        &now,
    )
    .await?;

    flash_ok(
        &session,
        if node_type == KbNode::TYPE_FOLDER {
            "文件夹已创建"
        } else {
            "文档已创建"
        },
    )
    .await?;
    Ok(Ok(see_other(&format!("/admin/docs/{book_id}?id={id}"))))
}

pub async fn admin_node_save(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<SaveNodeForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(node) = find_kb_node_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("页面不存在"));
    };
    let Some(book) = find_kb_book_by_id(&state.pool, node.book_id).await? else {
        return Err(AppError::not_found("知识库不存在"));
    };

    let title = form.title.trim().chars().take(120).collect::<String>();
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    if node.is_folder() {
        if title.is_empty() {
            return Err(AppError::bad_request("标题不能为空"));
        }
        rename_kb_node(&state.pool, id, &title, &now).await?;
        flash_ok(&session, "文件夹已保存").await?;
        return Ok(Ok(see_other(&format!("/admin/docs/{}?id={id}", book.id))));
    }

    let slug = normalize_slug(&title, &form.slug);
    let status = status_value(&form.status);
    let visibility = visibility_value(&form.visibility);
    let content_md = form.content_md;
    let content_html = render_markdown(&content_md);
    let published_at = timefmt::resolve_published_input(&form.published_at);
    let published_at_local = timefmt::form_local_or(&published_at, &form.published_at);

    let nodes = list_kb_nodes_by_book(&state.pool, book.id).await?;
    let tree = build_kb_tree(&nodes);

    let fail = |msg: String| -> AppResult<Html<String>> {
        render(AdminDocsBookTemplate {
            settings: settings.clone(),
            username: user.username.clone(),
            book: book.clone(),
            tree_html: tree_html(&tree, Some(id), book.id, &book.slug, true),
            has_nodes: !nodes.is_empty(),
            selected_id: Some(id),
            is_folder: false,
            title: title.clone(),
            slug: slug.clone(),
            content_md: content_md.clone(),
            status: status.to_string(),
            visibility: visibility.to_string(),
            published_at_local: published_at_local.clone(),
            form_action: format!("/admin/docs/nodes/{id}/save"),
            catalog_html: String::new(),
            doc_count: 0,
            word_count: 0,
            error: Some(msg),
        })
    };

    if title.is_empty() {
        return Ok(Err(fail("标题不能为空".into())?));
    }
    if kb_node_slug_taken(&state.pool, book.id, &slug, Some(id)).await? {
        return Ok(Err(fail("Slug 已被占用".into())?));
    }

    update_kb_node(
        &state.pool,
        id,
        &title,
        &slug,
        &content_md,
        &content_html,
        status,
        visibility,
        Some(&published_at),
        &now,
    )
    .await
    .map_err(|_| AppError::bad_request("保存失败，请稍后重试"))?;

    flash_ok(&session, "已保存").await?;
    Ok(Ok(see_other(&format!("/admin/docs/{}?id={id}", book.id))))
}

pub async fn admin_node_rename(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<RenameNodeForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    match require_user(&state, &session).await {
        Ok(_) => {}
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(node) = find_kb_node_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("页面不存在"));
    };
    let title = form.title.trim().chars().take(120).collect::<String>();
    if title.is_empty() {
        return Err(AppError::bad_request("标题不能为空"));
    }
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    rename_kb_node(&state.pool, id, &title, &now).await?;
    flash_ok(&session, "已重命名").await?;
    Ok(Ok(see_other(&format!(
        "/admin/docs/{}?id={id}",
        node.book_id
    ))))
}

pub async fn admin_node_delete(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Result<Redirect, Html<String>>> {
    match require_user(&state, &session).await {
        Ok(_) => {}
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(node) = find_kb_node_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("页面不存在"));
    };
    let book_id = node.book_id;
    let parent_id = node.parent_id;
    delete_kb_node(&state.pool, id).await?;
    flash_ok(&session, "已删除").await?;
    let redirect = match parent_id {
        Some(pid) => format!("/admin/docs/{book_id}?id={pid}"),
        None => format!("/admin/docs/{book_id}"),
    };
    Ok(Ok(see_other(&redirect)))
}

pub async fn admin_node_move(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<MoveNodeForm>,
) -> AppResult<axum::http::StatusCode> {
    match require_user(&state, &session).await {
        Ok(_) => {}
        Err(e) => return Err(cloak_auth(e)),
    };
    let Some(_) = find_kb_node_by_id(&state.pool, id).await? else {
        return Err(AppError::not_found("页面不存在"));
    };
    let parent_id = parse_parent_id(&form.parent_id);
    let before_id = parse_parent_id(&form.before_id);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    move_kb_node(&state.pool, id, parent_id, before_id, &now).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
