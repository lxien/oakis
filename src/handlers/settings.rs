use axum::Form;
use axum::extract::{Query, RawForm, State};
use axum::response::{Html, Redirect};
use serde::Deserialize;
use std::collections::HashMap;
use tower_sessions::Session;

use crate::infra::error::{AppResult, render, see_other};
use crate::infra::flash::{flash_err, flash_ok};
use crate::infra::password::{hash_password, validate_password_len, verify_password};
use crate::infra::state::AppState;
use crate::infra::upload::delete_site_image;
use crate::models::{SiteSettings, SocialLink};
use crate::store::{save_site_settings, update_user_password};
use crate::views::AdminSettingsTemplate;
use crate::web::authz::{cloak_auth, require_user};

const MAX_TITLE: usize = 80;
const MAX_TAGLINE: usize = 160;
const MAX_AUTHOR: usize = 40;
const MAX_KEYWORDS: usize = 200;
const MAX_DESCRIPTION: usize = 300;
const MAX_BEIAN: usize = 80;
const MAX_FOOTER_TEXT: usize = 160;

fn clamp_text(input: &str, max: usize) -> String {
    input.trim().chars().take(max).collect()
}

fn form_map(bytes: &[u8]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (k, v) in form_urlencoded::parse(bytes) {
        map.entry(k.into_owned()).or_default().push(v.into_owned());
    }
    map
}

fn form_one(map: &HashMap<String, Vec<String>>, key: &str) -> String {
    map.get(key)
        .and_then(|v| v.last())
        .cloned()
        .unwrap_or_default()
}

fn form_many(map: &HashMap<String, Vec<String>>, key: &str) -> Vec<String> {
    map.get(key).cloned().unwrap_or_default()
}

fn assemble_social_links(
    kinds: &[String],
    labels: &[String],
    urls: &[String],
) -> Result<Vec<SocialLink>, String> {
    let n = kinds.len().max(labels.len()).max(urls.len());
    if n > 12 {
        return Err("社交链接最多 12 条".into());
    }

    let mut out = Vec::new();
    for i in 0..n {
        let kind = kinds.get(i).map(|s| s.as_str()).unwrap_or("custom");
        let label = labels.get(i).map(|s| s.as_str()).unwrap_or("");
        let url = urls.get(i).map(|s| s.as_str()).unwrap_or("");
        if url.trim().is_empty() {
            continue;
        }
        let kind = SocialLink::normalize_kind(kind);
        let Some(url) = SocialLink::sanitize_url(url, &kind) else {
            continue;
        };
        let mut label: String = label.trim().chars().take(40).collect();
        if label.is_empty() {
            label = SocialLink::default_label(&kind).into();
        }
        out.push(SocialLink { kind, label, url });
    }
    Ok(out)
}

async fn replace_brand_url(current: &mut String, next_raw: &str, site_dir: &std::path::Path) {
    let next = SiteSettings::sanitize_asset_url(next_raw);
    if next == *current {
        return;
    }
    if current.starts_with("/uploads/site/") {
        delete_site_image(current, site_dir).await;
    }
    *current = next;
}

pub async fn settings_page(
    State(state): State<AppState>,
    session: Session,
    Query(_query): Query<HashMap<String, String>>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    Ok(Err(render(AdminSettingsTemplate {
        settings,
        username: user.username,
        error: None,
        saved: false,
    })?))
}

pub async fn settings_save(
    State(state): State<AppState>,
    session: Session,
    RawForm(bytes): RawForm,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, mut settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let fail = |settings: SiteSettings, username: String, message: String| {
        render(AdminSettingsTemplate {
            settings,
            username,
            error: Some(message),
            saved: false,
        })
    };

    let map = form_map(&bytes);
    let site_title = clamp_text(&form_one(&map, "site_title"), MAX_TITLE);
    let site_tagline = clamp_text(&form_one(&map, "site_tagline"), MAX_TAGLINE);
    let author_name = clamp_text(&form_one(&map, "author_name"), MAX_AUTHOR);
    let site_access = SiteSettings::normalize_site_access(&form_one(&map, "site_access"));
    let seo_keywords = clamp_text(&form_one(&map, "seo_keywords"), MAX_KEYWORDS);
    let seo_description = clamp_text(&form_one(&map, "seo_description"), MAX_DESCRIPTION);
    let icp_beian = clamp_text(&form_one(&map, "icp_beian"), MAX_BEIAN);
    let gongan_beian = clamp_text(&form_one(&map, "gongan_beian"), MAX_BEIAN);
    let gongan_beian_url =
        SiteSettings::sanitize_beian_url(&clamp_text(&form_one(&map, "gongan_beian_url"), 500));
    let footer_copyright = clamp_text(&form_one(&map, "footer_copyright"), MAX_FOOTER_TEXT);
    let footer_text = clamp_text(&form_one(&map, "footer_text"), MAX_FOOTER_TEXT);
    let logo_url = form_one(&map, "logo_url");
    let avatar_url = form_one(&map, "avatar_url");
    let favicon_url = form_one(&map, "favicon_url");
    let social_kind = form_many(&map, "social_kind");
    let social_label = form_many(&map, "social_label");
    let social_url = form_many(&map, "social_url");

    if site_title.is_empty() {
        return Ok(Err(fail(
            settings,
            user.username,
            "站点标题不能为空".into(),
        )?));
    }
    if author_name.is_empty() {
        return Ok(Err(fail(settings, user.username, "作者名不能为空".into())?));
    }

    let social_links = match assemble_social_links(&social_kind, &social_label, &social_url) {
        Ok(v) => v,
        Err(msg) => {
            settings.site_title = site_title;
            settings.site_tagline = site_tagline;
            settings.author_name = author_name;
            settings.site_access = site_access;
            return Ok(Err(fail(settings, user.username, msg)?));
        }
    };

    settings.site_title = site_title;
    settings.site_tagline = site_tagline;
    settings.author_name = author_name;
    settings.site_access = site_access;
    settings.seo_keywords = seo_keywords;
    settings.seo_description = seo_description;
    settings.icp_beian = icp_beian;
    settings.gongan_beian = gongan_beian;
    settings.gongan_beian_url = gongan_beian_url;
    settings.footer_copyright = footer_copyright;
    settings.footer_text = footer_text;
    settings.social_links = social_links;

    replace_brand_url(
        &mut settings.avatar_url,
        &avatar_url,
        &state.config.site_dir(),
    )
    .await;
    replace_brand_url(&mut settings.logo_url, &logo_url, &state.config.site_dir()).await;
    replace_brand_url(
        &mut settings.favicon_url,
        &favicon_url,
        &state.config.site_dir(),
    )
    .await;

    save_site_settings(&state.pool, &settings).await?;
    flash_ok(&session, "设置已保存").await?;
    Ok(Ok(see_other("/admin/settings")))
}

#[derive(Deserialize)]
pub struct PasswordForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

pub async fn settings_password(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<PasswordForm>,
) -> AppResult<Redirect> {
    let (user, _settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    if let Err(msg) = validate_password_len(&form.new_password) {
        flash_err(&session, msg).await?;
        return Ok(see_other("/admin/settings"));
    }
    if form.new_password != form.confirm_password {
        flash_err(&session, "两次输入的新密码不一致").await?;
        return Ok(see_other("/admin/settings"));
    }
    if !verify_password(&form.current_password, &user.password_hash)? {
        flash_err(&session, "当前密码错误").await?;
        return Ok(see_other("/admin/settings"));
    }

    let hash = hash_password(&form.new_password)?;
    update_user_password(&state.pool, user.id, &hash).await?;
    session.flush().await?;
    flash_ok(&session, "密码已更新，请重新登录").await?;
    Ok(see_other(&state.config.login_path))
}
