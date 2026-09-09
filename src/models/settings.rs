use serde::{Deserialize, Serialize};

use super::nav::NavItemConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialLink {
    pub kind: String,
    pub label: String,
    pub url: String,
}

impl SocialLink {
    pub const KINDS: &'static [(&'static str, &'static str)] = &[
        ("github", "GitHub"),
        ("gitee", "Gitee"),
        ("site", "个人站"),
        ("email", "邮箱"),
        ("twitter", "X / Twitter"),
        ("bilibili", "Bilibili"),
        ("rss", "RSS"),
        ("custom", "自定义"),
    ];

    pub fn default_label(kind: &str) -> &'static str {
        Self::KINDS
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, label)| *label)
            .unwrap_or("链接")
    }

    pub fn normalize_kind(kind: &str) -> String {
        let kind = kind.trim().to_ascii_lowercase();
        if Self::KINDS.iter().any(|(k, _)| *k == kind) {
            kind
        } else {
            "custom".into()
        }
    }

    pub fn sanitize_url(raw: &str, kind: &str) -> Option<String> {
        let url = raw.trim();
        if url.is_empty() || url.len() > 500 {
            return None;
        }
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("javascript:")
            || lower.starts_with("data:")
            || lower.starts_with("vbscript:")
        {
            return None;
        }

        if kind == "email" || lower.starts_with("mailto:") {
            if lower.starts_with("mailto:") {
                return Some(url.to_string());
            }

            if url.contains('@') && !url.contains(char::is_whitespace) {
                return Some(format!("mailto:{url}"));
            }
            return Some(url.to_string());
        }

        if lower.starts_with("https://") || lower.starts_with("http://") {
            return Some(url.to_string());
        }

        if url.starts_with('/') && !url.starts_with("//") {
            return Some(url.to_string());
        }

        if url.contains('.') && !url.contains(char::is_whitespace) {
            return Some(format!("https://{url}"));
        }
        Some(url.to_string())
    }

    pub fn parse_list(json: &str) -> Vec<Self> {
        let Ok(raw) = serde_json::from_str::<Vec<SocialLink>>(json) else {
            return Vec::new();
        };
        raw.into_iter()
            .filter_map(|item| {
                let kind = Self::normalize_kind(&item.kind);
                let url = Self::sanitize_url(&item.url, &kind)?;
                let mut label: String = item.label.trim().chars().take(40).collect();
                if label.is_empty() {
                    label = Self::default_label(&kind).into();
                }
                Some(Self { kind, label, url })
            })
            .take(12)
            .collect()
    }

    pub fn is_github(&self) -> bool {
        self.kind == "github"
    }
    pub fn is_gitee(&self) -> bool {
        self.kind == "gitee"
    }
    pub fn is_site(&self) -> bool {
        self.kind == "site"
    }
    pub fn is_email(&self) -> bool {
        self.kind == "email"
    }
    pub fn is_twitter(&self) -> bool {
        self.kind == "twitter"
    }
    pub fn is_bilibili(&self) -> bool {
        self.kind == "bilibili"
    }
    pub fn is_rss(&self) -> bool {
        self.kind == "rss"
    }

    pub fn icon_src(&self) -> &'static str {
        match self.kind.as_str() {
            "github" => "/static/icons/github.svg",
            "gitee" => "/static/icons/gitee.svg",
            "site" => "/static/icons/site.svg",
            "email" => "/static/icons/email.svg",
            "bilibili" => "/static/icons/bilibili.svg",
            "rss" => "/static/icons/rss.svg",
            "twitter" => "/static/icons/web.svg",
            _ => "/static/icons/web.svg",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteSettings {
    pub site_title: String,
    pub site_tagline: String,
    pub author_name: String,
    pub avatar_url: String,
    pub logo_url: String,
    pub favicon_url: String,
    pub seo_keywords: String,
    pub seo_description: String,
    pub social_links: Vec<SocialLink>,

    pub nav_items: Vec<NavItemConfig>,

    pub icp_beian: String,

    pub gongan_beian: String,

    pub gongan_beian_url: String,

    pub footer_copyright: String,

    pub footer_text: String,

    pub site_access: String,
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            site_title: "oakis".into(),
            site_tagline: String::new(),
            author_name: String::new(),
            avatar_url: String::new(),
            logo_url: String::new(),
            favicon_url: String::new(),
            seo_keywords: String::new(),
            seo_description: String::new(),
            social_links: Vec::new(),
            nav_items: NavItemConfig::system_defaults(),
            icp_beian: String::new(),
            gongan_beian: String::new(),
            gongan_beian_url: String::new(),
            footer_copyright: String::new(),
            footer_text: String::new(),
            site_access: "public".into(),
        }
    }
}

impl SiteSettings {
    pub const ACCESS_PUBLIC: &'static str = "public";
    pub const ACCESS_MAINTENANCE: &'static str = "maintenance";
    pub const ACCESS_PRIVATE: &'static str = "private";

    pub fn normalize_site_access(raw: &str) -> String {
        match raw.trim() {
            Self::ACCESS_MAINTENANCE => Self::ACCESS_MAINTENANCE.into(),
            Self::ACCESS_PRIVATE => Self::ACCESS_PRIVATE.into(),
            _ => Self::ACCESS_PUBLIC.into(),
        }
    }

    pub fn is_access_public(&self) -> bool {
        self.site_access == Self::ACCESS_PUBLIC
    }

    pub fn is_access_maintenance(&self) -> bool {
        self.site_access == Self::ACCESS_MAINTENANCE
    }

    pub fn is_access_private(&self) -> bool {
        self.site_access == Self::ACCESS_PRIVATE
    }

    pub fn sanitize_asset_url(url: &str) -> String {
        crate::infra::upload::sanitize_brand_url(url)
    }

    pub fn has_social_links(&self) -> bool {
        !self.social_links.is_empty()
    }

    pub fn profile_avatar_url(&self) -> &str {
        if !self.avatar_url.is_empty() {
            self.avatar_url.as_str()
        } else if !self.logo_url.is_empty() {
            self.logo_url.as_str()
        } else {
            ""
        }
    }

    pub fn has_profile_avatar(&self) -> bool {
        !self.profile_avatar_url().is_empty()
    }

    pub fn social_links_json(&self) -> String {
        serde_json::to_string(&self.social_links).unwrap_or_else(|_| "[]".into())
    }

    pub fn has_icp_beian(&self) -> bool {
        !self.icp_beian.trim().is_empty()
    }

    pub fn has_gongan_beian(&self) -> bool {
        !self.gongan_beian.trim().is_empty()
    }

    pub fn has_footer_copyright(&self) -> bool {
        !self.footer_copyright.trim().is_empty()
    }

    pub fn has_footer_text(&self) -> bool {
        !self.footer_text.trim().is_empty()
    }

    pub fn sanitize_beian_url(raw: &str) -> String {
        let url = raw.trim();
        if url.is_empty() || url.len() > 500 {
            return String::new();
        }
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("https://") || lower.starts_with("http://") {
            url.to_string()
        } else {
            String::new()
        }
    }
}
