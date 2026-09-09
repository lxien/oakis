use axum::Json;
use axum::extract::State;
use serde::Serialize;
use tower_sessions::Session;

use crate::infra::error::AppResult;
use crate::infra::flash::take_flash;
use crate::infra::state::AppState;
use crate::web::authz::require_user;

#[derive(Serialize)]
pub struct FlashPayload {
    pub level: String,
    pub message: String,
}

pub async fn admin_flash(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Json<Option<FlashPayload>>> {
    if require_user(&state, &session).await.is_err() {
        return Ok(Json(None));
    }

    let payload = take_flash(&session).await?.map(|f| FlashPayload {
        level: f.level,
        message: f.message,
    });
    Ok(Json(payload))
}
