use axum::extract::State;
use axum::response::{Html, Redirect};
use tower_sessions::Session;

use crate::infra::error::{AppResult, render};
use crate::infra::state::AppState;
use crate::store::load_dashboard;
use crate::views::AdminDashboardTemplate;
use crate::web::authz::{cloak_auth, require_user};

pub async fn dashboard(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    Ok(Err(render(AdminDashboardTemplate {
        settings,
        username: user.username,
        dash: load_dashboard(&state.pool).await?,
    })?))
}
