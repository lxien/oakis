use askama::Template;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Redirect, Response};
use tower_sessions::Session;

use crate::infra::error::AppError;
use crate::infra::state::AppState;
use crate::store::load_settings;
use crate::web::authz::session_logged_in;

const ALLOWED_WHEN_NOT_INSTALLED: &[&str] = &["/install", "/static"];

pub async fn require_installed(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    let installed = *state.installed.read().await;

    if !installed {
        let allowed = ALLOWED_WHEN_NOT_INSTALLED
            .iter()
            .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")));
        if !allowed {
            return Redirect::temporary("/install").into_response();
        }
    } else if path == "/install" || path.starts_with("/install/") {
        return Redirect::temporary("/").into_response();
    }

    next.run(request).await
}

fn is_static_asset(path: &str) -> bool {
    path == "/static" || path.starts_with("/static/")
}

fn is_upload_asset(path: &str) -> bool {
    path == "/uploads" || path.starts_with("/uploads/")
}

fn is_admin_path(path: &str) -> bool {
    path == "/admin" || path.starts_with("/admin/")
}

fn cloaked_404() -> Response {
    AppError::not_found("页面不存在").into_response()
}

#[derive(Template)]
#[template(path = "themes/default/maintenance.html")]
struct MaintenanceTemplate {}

pub async fn site_access_gate(
    State(state): State<AppState>,
    session: Session,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    let login_path = state.config.login_path.as_str();

    if path == login_path || path.starts_with(&format!("{login_path}/")) || path == "/logout" {
        return next.run(request).await;
    }

    if is_static_asset(path) {
        return next.run(request).await;
    }

    let logged_in = match session_logged_in(&state, &session).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };

    if is_admin_path(path) {
        if logged_in {
            return next.run(request).await;
        }
        return cloaked_404();
    }

    let settings = match load_settings(&state.pool).await {
        Ok(s) => s,
        Err(e) => return e.into_response(),
    };

    if is_upload_asset(path) {
        if settings.is_access_public() || logged_in {
            return next.run(request).await;
        }
        return cloaked_404();
    }

    if settings.is_access_public() {
        return next.run(request).await;
    }

    if logged_in {
        return next.run(request).await;
    }

    if settings.is_access_maintenance() {
        let body = MaintenanceTemplate {}
            .render()
            .unwrap_or_else(|_| "站点维护中".into());
        return (StatusCode::SERVICE_UNAVAILABLE, Html(body)).into_response();
    }

    cloaked_404()
}
