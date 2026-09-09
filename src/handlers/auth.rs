use axum::Form;
use axum::extract::{ConnectInfo, Query, State};
use axum::http::HeaderMap;
use axum::response::{Html, Redirect};
use serde::Deserialize;
use std::net::SocketAddr;
use tower_sessions::Session;

use crate::infra::auth_limit::LoginLimiter;
use crate::infra::error::{AppResult, render, see_other};
use crate::infra::password::verify_password;
use crate::infra::state::AppState;
use crate::store::{find_user_by_username, load_settings};
use crate::views::LoginTemplate;
use crate::web::authz::{clear_user, session_logged_in, set_user_id};

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    pub next: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginQuery {
    pub next: Option<String>,
}

fn safe_next(next: Option<&str>) -> String {
    let Some(n) = next.map(str::trim).filter(|s| !s.is_empty()) else {
        return String::new();
    };
    if n.starts_with('/')
        && !n.starts_with("//")
        && !n.contains('\\')
        && !n.contains('\0')
        && !n.contains('\n')
        && !n.contains('\r')
    {
        n.to_string()
    } else {
        String::new()
    }
}

fn after_login(next: &str) -> String {
    if next.is_empty() {
        "/admin".into()
    } else {
        next.to_string()
    }
}

fn client_key(headers: &HeaderMap, addr: SocketAddr, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(xff) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return xff.to_string();
        }
        if let Some(real) = headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return real.to_string();
        }
    }
    addr.ip().to_string()
}

pub async fn login_page(
    State(state): State<AppState>,
    session: Session,
    Query(q): Query<LoginQuery>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let next = safe_next(q.next.as_deref());
    if session_logged_in(&state, &session).await? {
        return Ok(Ok(see_other(&after_login(&next))));
    }

    let settings = load_settings(&state.pool).await?;
    Ok(Err(render(LoginTemplate {
        settings,
        login_path: state.config.login_path.clone(),
        error: None,
        username: String::new(),
        next,
    })?))
}

pub async fn login_submit(
    State(state): State<AppState>,
    session: Session,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let settings = load_settings(&state.pool).await?;
    let username = form.username.trim().to_string();
    let login_path = state.config.login_path.clone();
    let next = safe_next(form.next.as_deref());
    let client = client_key(&headers, addr, state.config.trust_proxy);
    let limit_key = LoginLimiter::key(&client, &username);

    let fail = |message: &str| -> AppResult<Result<Redirect, Html<String>>> {
        Ok(Err(render(LoginTemplate {
            settings: settings.clone(),
            login_path: login_path.clone(),
            error: Some(message.to_string()),
            username: username.clone(),
            next: next.clone(),
        })?))
    };

    if state.login_limiter.is_blocked(&limit_key) {
        return fail("尝试过多，请稍后再试");
    }

    let Some(user) = find_user_by_username(&state.pool, &username).await? else {
        state.login_limiter.record_failure(&limit_key);
        return fail("用户名或密码错误");
    };

    if !verify_password(&form.password, &user.password_hash)? {
        state.login_limiter.record_failure(&limit_key);
        return fail("用户名或密码错误");
    }

    state.login_limiter.clear(&limit_key);

    session.flush().await?;
    set_user_id(&session, user.id).await?;
    Ok(Ok(see_other(&after_login(&next))))
}

pub async fn logout(session: Session) -> AppResult<Redirect> {
    clear_user(&session).await?;
    session.flush().await?;
    Ok(see_other("/"))
}
