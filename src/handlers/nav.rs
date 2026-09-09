use axum::extract::{RawForm, State};
use axum::response::{Html, Redirect};
use std::collections::HashMap;
use tower_sessions::Session;

use crate::infra::error::{AppResult, render, see_other};
use crate::infra::flash::flash_ok;
use crate::infra::state::AppState;
use crate::models::NavItemConfig;
use crate::models::nav::sanitize_nav_url;
use crate::store::{list_published_page_refs, save_nav_items};
use crate::views::AdminNavTemplate;
use crate::web::authz::{cloak_auth, require_user};

fn form_map(bytes: &[u8]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (k, v) in form_urlencoded::parse(bytes) {
        map.entry(k.into_owned()).or_default().push(v.into_owned());
    }
    map
}

fn form_many(map: &HashMap<String, Vec<String>>, key: &str) -> Vec<String> {
    map.get(key).cloned().unwrap_or_default()
}

fn assemble_nav_items(
    map: &HashMap<String, Vec<String>>,
) -> Result<Vec<NavItemConfig>, (String, Vec<NavItemConfig>)> {
    let kinds = form_many(map, "nav_type");
    let keys = form_many(map, "nav_key");
    let page_ids = form_many(map, "nav_page_id");
    let labels = form_many(map, "nav_label");
    let urls = form_many(map, "nav_url");
    let enabled_flags = form_many(map, "nav_enabled");

    let n = kinds
        .len()
        .max(keys.len())
        .max(page_ids.len())
        .max(labels.len())
        .max(urls.len())
        .max(enabled_flags.len());
    if n > NavItemConfig::MAX_ITEMS {
        return Err((
            format!("导航最多 {} 项", NavItemConfig::MAX_ITEMS),
            NavItemConfig::system_defaults(),
        ));
    }

    let mut raw = Vec::with_capacity(n);
    let mut err: Option<String> = None;

    for i in 0..n {
        let kind = kinds.get(i).map(|s| s.as_str()).unwrap_or("builtin");
        let key = keys.get(i).map(|s| s.as_str()).unwrap_or("").to_string();
        let page_id = page_ids
            .get(i)
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|id| *id > 0);
        let label = labels.get(i).cloned().unwrap_or_default();
        let url = urls.get(i).cloned().unwrap_or_default();
        let enabled = enabled_flags
            .get(i)
            .map(|s| s == "1" || s == "on" || s == "true")
            .unwrap_or(true);

        match kind {
            "page" if page_id.is_none() && err.is_none() => {
                err = Some("请为页面导航选择页面".into());
            }
            "custom" if sanitize_nav_url(&url).is_none() && err.is_none() => {
                err = Some("请填写有效链接（https:// 或 /path）".into());
            }
            _ => {}
        }

        raw.push(NavItemConfig {
            kind: kind.into(),
            key,
            page_id,
            label,
            url,
            enabled,
        });
    }

    let echo = NavItemConfig::for_editor(raw.clone());
    if let Some(msg) = err {
        return Err((msg, echo));
    }

    Ok(NavItemConfig::normalize_list(raw))
}

fn render_nav_page(
    settings: crate::models::SiteSettings,
    username: String,
    pages: Vec<crate::models::NavPageRef>,
    items: Vec<NavItemConfig>,
    error: Option<String>,
) -> AppResult<Html<String>> {
    render(AdminNavTemplate {
        settings,
        username,
        items,
        pages,
        error,
    })
}

pub async fn nav_page(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let pages = list_published_page_refs(&state.pool).await?;
    let items = NavItemConfig::normalize_list(settings.nav_items.clone());
    Ok(Err(render_nav_page(
        settings,
        user.username,
        pages,
        items,
        None,
    )?))
}

pub async fn nav_save(
    State(state): State<AppState>,
    session: Session,
    RawForm(bytes): RawForm,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    let pages = list_published_page_refs(&state.pool).await?;
    let map = form_map(&bytes);

    match assemble_nav_items(&map) {
        Ok(items) => {
            save_nav_items(&state.pool, &items).await?;
            flash_ok(&session, "导航已保存").await?;
            Ok(Ok(see_other("/admin/nav")))
        }
        Err((msg, echo)) => Ok(Err(render_nav_page(
            settings,
            user.username,
            pages,
            echo,
            Some(msg),
        )?)),
    }
}
