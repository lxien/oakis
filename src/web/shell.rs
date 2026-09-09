use tower_sessions::Session;

use crate::infra::error::AppResult;
use crate::infra::state::AppState;
use crate::models::PublicShell;
use crate::models::nav::resolve_nav_items;
use crate::store::{list_published_page_refs, load_settings};
use crate::web::authz::session_logged_in;

pub async fn load_public_shell(
    state: &AppState,
    session: &Session,
    path: &str,
    search_query: impl Into<String>,
) -> AppResult<PublicShell> {
    let logged_in = session_logged_in(state, session).await?;
    let settings = load_settings(&state.pool).await?;
    let pages = list_published_page_refs(&state.pool).await?;
    let nav_items = resolve_nav_items(&settings.nav_items, &pages, logged_in, path);
    Ok(PublicShell {
        settings,
        logged_in,
        nav_items,
        search_query: search_query.into(),
    })
}
