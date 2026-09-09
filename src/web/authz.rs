use tower_sessions::Session;

use crate::infra::error::{AppError, AppResult};
use crate::infra::state::AppState;
use crate::models::{SiteSettings, User};
use crate::store::{find_user_by_id, load_settings};

const SESSION_USER_KEY: &str = "user_id";

pub async fn current_user_id(session: &Session) -> AppResult<Option<i64>> {
    Ok(session.get::<i64>(SESSION_USER_KEY).await?)
}

pub async fn session_logged_in(state: &AppState, session: &Session) -> AppResult<bool> {
    let Some(user_id) = current_user_id(session).await? else {
        return Ok(false);
    };
    if find_user_by_id(&state.pool, user_id).await?.is_some() {
        return Ok(true);
    }
    let _ = clear_user(session).await;
    Ok(false)
}

pub async fn set_user_id(session: &Session, user_id: i64) -> AppResult<()> {
    session.insert(SESSION_USER_KEY, user_id).await?;
    Ok(())
}

pub async fn clear_user(session: &Session) -> AppResult<()> {
    session.remove::<i64>(SESSION_USER_KEY).await?;
    Ok(())
}

pub async fn require_user(state: &AppState, session: &Session) -> AppResult<(User, SiteSettings)> {
    let Some(user_id) = current_user_id(session).await? else {
        return Err(AppError::Unauthorized);
    };
    let Some(user) = find_user_by_id(&state.pool, user_id).await? else {
        return Err(AppError::Unauthorized);
    };
    let settings = load_settings(&state.pool).await?;
    Ok((user, settings))
}

pub fn cloak_auth(err: AppError) -> AppError {
    match err {
        AppError::Unauthorized => AppError::not_found("页面不存在"),
        other => other,
    }
}
