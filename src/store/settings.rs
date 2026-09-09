use sqlx::{SqlitePool, query, query_as, query_scalar};

use crate::infra::error::AppResult;
use crate::models::{NavItemConfig, SiteSettings};

pub async fn load_settings(pool: &SqlitePool) -> AppResult<SiteSettings> {
    let rows: Vec<(String, String)> = query_as("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await?;

    let mut settings = SiteSettings::default();
    for (key, value) in rows {
        match key.as_str() {
            "site_title" => settings.site_title = value,
            "site_tagline" => settings.site_tagline = value,
            "author_name" => settings.author_name = value,
            "avatar_url" => settings.avatar_url = SiteSettings::sanitize_asset_url(&value),
            "logo_url" => settings.logo_url = SiteSettings::sanitize_asset_url(&value),
            "favicon_url" => settings.favicon_url = SiteSettings::sanitize_asset_url(&value),
            "seo_keywords" => settings.seo_keywords = value,
            "seo_description" => settings.seo_description = value,
            "icp_beian" => settings.icp_beian = value,
            "gongan_beian" => settings.gongan_beian = value,
            "gongan_beian_url" => {
                settings.gongan_beian_url = SiteSettings::sanitize_beian_url(&value);
            }
            "footer_copyright" => settings.footer_copyright = value,
            "footer_text" => settings.footer_text = value,
            "site_access" => settings.site_access = SiteSettings::normalize_site_access(&value),
            "social_links" => {
                settings.social_links = crate::models::SocialLink::parse_list(&value);
            }
            "nav_items" => {
                settings.nav_items = NavItemConfig::parse_list(&value);
            }
            _ => {}
        }
    }
    Ok(settings)
}

pub async fn upsert_setting<'e, E>(executor: E, key: &str, value: &str) -> AppResult<()>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    query(
        "INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn save_site_settings(pool: &SqlitePool, settings: &SiteSettings) -> AppResult<()> {
    let social_json = settings.social_links_json();
    for (key, value) in [
        ("site_title", settings.site_title.as_str()),
        ("site_tagline", settings.site_tagline.as_str()),
        ("author_name", settings.author_name.as_str()),
        ("avatar_url", settings.avatar_url.as_str()),
        ("logo_url", settings.logo_url.as_str()),
        ("favicon_url", settings.favicon_url.as_str()),
        ("seo_keywords", settings.seo_keywords.as_str()),
        ("seo_description", settings.seo_description.as_str()),
        ("icp_beian", settings.icp_beian.as_str()),
        ("gongan_beian", settings.gongan_beian.as_str()),
        ("gongan_beian_url", settings.gongan_beian_url.as_str()),
        ("footer_copyright", settings.footer_copyright.as_str()),
        ("footer_text", settings.footer_text.as_str()),
        ("site_access", settings.site_access.as_str()),
        ("social_links", social_json.as_str()),
    ] {
        upsert_setting(pool, key, value).await?;
    }
    Ok(())
}

pub async fn save_nav_items(pool: &SqlitePool, items: &[NavItemConfig]) -> AppResult<()> {
    let json = NavItemConfig::to_json(items);
    upsert_setting(pool, "nav_items", &json).await
}

pub async fn ensure_nav_items(pool: &SqlitePool) -> AppResult<()> {
    let existing: Option<String> =
        query_scalar("SELECT value FROM settings WHERE key = 'nav_items'")
            .fetch_optional(pool)
            .await?;
    if existing.is_some() {
        return Ok(());
    }

    let mut items = NavItemConfig::system_defaults();
    let page_ids: Vec<(i64,)> = query_as(
        "SELECT id FROM pages
         WHERE status = 'published' AND show_in_nav = 1
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    for (id,) in page_ids {
        items.push(NavItemConfig {
            kind: NavItemConfig::KIND_PAGE.into(),
            page_id: Some(id),
            enabled: true,
            ..Default::default()
        });
    }
    save_nav_items(pool, &items).await
}
