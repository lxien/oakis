use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct BuiltinRoute {
    pub key: &'static str,
    pub default_label: &'static str,
    pub href: &'static str,
}

pub const BUILTIN_ROUTES: &[BuiltinRoute] = &[
    BuiltinRoute {
        key: "home",
        default_label: "首页",
        href: "/",
    },
    BuiltinRoute {
        key: "posts",
        default_label: "文章",
        href: "/posts",
    },
    BuiltinRoute {
        key: "docs",
        default_label: "知识库",
        href: "/docs",
    },
    BuiltinRoute {
        key: "sparks",
        default_label: "灵感",
        href: "/sparks",
    },
];

impl BuiltinRoute {
    pub fn find(key: &str) -> Option<&'static BuiltinRoute> {
        BUILTIN_ROUTES.iter().find(|r| r.key == key)
    }

    pub fn is_active(&self, path: &str) -> bool {
        match self.key {
            "home" => path == "/",
            "posts" => {
                path == "/posts"
                    || path.starts_with("/posts/")
                    || path.starts_with("/categories/")
                    || path.starts_with("/tags/")
            }
            "docs" => path == "/docs" || path.starts_with("/docs/"),
            "sparks" => path == "/sparks" || path.starts_with("/sparks/"),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NavItemConfig {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub page_id: Option<i64>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub url: String,
    pub visibility: String,
}

impl<'de> Deserialize<'de> for NavItemConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        NavItemConfigDto::deserialize(deserializer).map(Into::into)
    }
}

#[derive(Debug, Deserialize)]
struct NavItemConfigDto {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    page_id: Option<i64>,
    #[serde(default)]
    label: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    visibility: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
}

impl From<NavItemConfigDto> for NavItemConfig {
    fn from(dto: NavItemConfigDto) -> Self {
        let visibility = match dto.visibility.as_deref() {
            Some(v) => NavItemConfig::normalize_visibility(v),
            None => match dto.enabled {
                Some(false) => NavItemConfig::VIS_HIDDEN.into(),
                _ => NavItemConfig::VIS_PUBLIC.into(),
            },
        };
        Self {
            kind: dto.kind,
            key: dto.key,
            page_id: dto.page_id,
            label: dto.label,
            url: dto.url,
            visibility,
        }
    }
}

impl Default for NavItemConfig {
    fn default() -> Self {
        Self {
            kind: Self::KIND_BUILTIN.into(),
            key: String::new(),
            page_id: None,
            label: String::new(),
            url: String::new(),
            visibility: Self::VIS_PUBLIC.into(),
        }
    }
}

impl NavItemConfig {
    pub const KIND_BUILTIN: &'static str = "builtin";
    pub const KIND_PAGE: &'static str = "page";
    pub const KIND_CUSTOM: &'static str = "custom";
    pub const MAX_ITEMS: usize = 16;

    pub const VIS_PUBLIC: &'static str = "public";
    pub const VIS_PRIVATE: &'static str = "private";
    pub const VIS_HIDDEN: &'static str = "hidden";

    pub fn normalize_visibility(raw: &str) -> String {
        match raw.trim() {
            Self::VIS_PRIVATE => Self::VIS_PRIVATE.into(),
            Self::VIS_HIDDEN => Self::VIS_HIDDEN.into(),
            _ => Self::VIS_PUBLIC.into(),
        }
    }

    pub fn is_public(&self) -> bool {
        self.visibility == Self::VIS_PUBLIC
    }

    pub fn is_private(&self) -> bool {
        self.visibility == Self::VIS_PRIVATE
    }

    pub fn is_hidden(&self) -> bool {
        self.visibility == Self::VIS_HIDDEN
    }

    pub fn is_visible_to(&self, logged_in: bool) -> bool {
        match self.visibility.as_str() {
            Self::VIS_HIDDEN => false,
            Self::VIS_PRIVATE => logged_in,
            _ => true,
        }
    }

    pub fn system_defaults() -> Vec<Self> {
        BUILTIN_ROUTES
            .iter()
            .map(|r| Self {
                kind: Self::KIND_BUILTIN.into(),
                key: r.key.into(),
                label: String::new(),
                visibility: Self::VIS_PUBLIC.into(),
                ..Default::default()
            })
            .collect()
    }

    pub fn is_builtin(&self) -> bool {
        self.kind == Self::KIND_BUILTIN
    }

    pub fn is_page(&self) -> bool {
        self.kind == Self::KIND_PAGE
    }

    pub fn is_custom(&self) -> bool {
        self.kind == Self::KIND_CUSTOM
    }

    pub fn page_id_value(&self) -> i64 {
        self.page_id.unwrap_or(0)
    }

    pub fn builtin_default_label(&self) -> &'static str {
        BuiltinRoute::find(&self.key)
            .map(|r| r.default_label)
            .unwrap_or("")
    }

    pub fn parse_list(json: &str) -> Vec<Self> {
        let Ok(raw) = serde_json::from_str::<Vec<NavItemConfigDto>>(json) else {
            return Self::system_defaults();
        };
        Self::normalize_list(raw.into_iter().map(Into::into).collect())
    }

    pub fn normalize_list(raw: Vec<Self>) -> Vec<Self> {
        let mut out = Vec::new();
        let mut seen_builtin = std::collections::HashSet::new();

        for item in raw.into_iter().take(Self::MAX_ITEMS) {
            let visibility = Self::normalize_visibility(&item.visibility);
            match item.kind.as_str() {
                Self::KIND_BUILTIN => {
                    let key = item.key.trim().to_ascii_lowercase();
                    if BuiltinRoute::find(&key).is_none() || !seen_builtin.insert(key.clone()) {
                        continue;
                    }
                    let label: String = item.label.trim().chars().take(40).collect();
                    out.push(Self {
                        kind: Self::KIND_BUILTIN.into(),
                        key,
                        page_id: None,
                        label,
                        url: String::new(),
                        visibility,
                    });
                }
                Self::KIND_PAGE => {
                    let Some(page_id) = item.page_id.filter(|id| *id > 0) else {
                        continue;
                    };
                    let label: String = item.label.trim().chars().take(40).collect();
                    out.push(Self {
                        kind: Self::KIND_PAGE.into(),
                        key: String::new(),
                        page_id: Some(page_id),
                        label,
                        url: String::new(),
                        visibility,
                    });
                }
                Self::KIND_CUSTOM => {
                    let Some(url) = sanitize_nav_url(&item.url) else {
                        continue;
                    };
                    let mut label: String = item.label.trim().chars().take(40).collect();
                    if label.is_empty() {
                        label = "链接".into();
                    }
                    out.push(Self {
                        kind: Self::KIND_CUSTOM.into(),
                        key: String::new(),
                        page_id: None,
                        label,
                        url,
                        visibility,
                    });
                }
                _ => {}
            }
        }

        Self::ensure_builtins(out)
    }

    pub fn for_editor(raw: Vec<Self>) -> Vec<Self> {
        let mut out = Vec::new();
        let mut seen_builtin = std::collections::HashSet::new();

        for item in raw.into_iter().take(Self::MAX_ITEMS) {
            let visibility = Self::normalize_visibility(&item.visibility);
            match item.kind.as_str() {
                Self::KIND_BUILTIN => {
                    let key = item.key.trim().to_ascii_lowercase();
                    if BuiltinRoute::find(&key).is_none() || !seen_builtin.insert(key.clone()) {
                        continue;
                    }
                    let label: String = item.label.trim().chars().take(40).collect();
                    out.push(Self {
                        kind: Self::KIND_BUILTIN.into(),
                        key,
                        page_id: None,
                        label,
                        url: String::new(),
                        visibility,
                    });
                }
                Self::KIND_PAGE => {
                    let label: String = item.label.trim().chars().take(40).collect();
                    out.push(Self {
                        kind: Self::KIND_PAGE.into(),
                        key: String::new(),
                        page_id: item.page_id.filter(|id| *id > 0),
                        label,
                        url: String::new(),
                        visibility,
                    });
                }
                Self::KIND_CUSTOM => {
                    let mut label: String = item.label.trim().chars().take(40).collect();
                    if label.is_empty() {
                        label = "链接".into();
                    }
                    let url: String = item.url.trim().chars().take(500).collect();
                    out.push(Self {
                        kind: Self::KIND_CUSTOM.into(),
                        key: String::new(),
                        page_id: None,
                        label,
                        url,
                        visibility,
                    });
                }
                _ => {}
            }
        }

        Self::ensure_builtins(out)
    }

    fn ensure_builtins(mut items: Vec<Self>) -> Vec<Self> {
        let present: std::collections::HashSet<String> = items
            .iter()
            .filter(|i| i.is_builtin())
            .map(|i| i.key.clone())
            .collect();

        let missing: Vec<Self> = BUILTIN_ROUTES
            .iter()
            .filter(|r| !present.contains(r.key))
            .map(|r| Self {
                kind: Self::KIND_BUILTIN.into(),
                key: r.key.into(),
                visibility: Self::VIS_PUBLIC.into(),
                ..Default::default()
            })
            .collect();

        if missing.is_empty() {
            if items.is_empty() {
                return Self::system_defaults();
            }
            return items;
        }

        missing.into_iter().chain(items.drain(..)).collect()
    }

    pub fn to_json(items: &[Self]) -> String {
        serde_json::to_string(items).unwrap_or_else(|_| "[]".into())
    }
}

#[derive(Debug, Clone)]
pub struct NavPageRef {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub visibility: String,
}

impl NavPageRef {
    pub fn is_private(&self) -> bool {
        self.visibility == "private"
    }

    pub fn href(&self) -> String {
        format!("/p/{}", self.slug)
    }
}

#[derive(Debug, Clone)]
pub struct NavItemView {
    pub label: String,
    pub href: String,
    pub active: bool,
    pub external: bool,
}

impl NavItemView {
    pub fn is_external(&self) -> bool {
        self.external
    }
}

#[derive(Debug, Clone)]
pub struct PublicShell {
    pub settings: crate::models::SiteSettings,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
}

pub fn resolve_nav_items(
    configs: &[NavItemConfig],
    pages: &[NavPageRef],
    logged_in: bool,
    path: &str,
) -> Vec<NavItemView> {
    let mut out = Vec::with_capacity(configs.len());

    for cfg in configs {
        if !cfg.is_visible_to(logged_in) {
            continue;
        }
        match cfg.kind.as_str() {
            NavItemConfig::KIND_BUILTIN => {
                let Some(route) = BuiltinRoute::find(&cfg.key) else {
                    continue;
                };
                let label = if cfg.label.trim().is_empty() {
                    route.default_label.to_string()
                } else {
                    cfg.label.clone()
                };
                out.push(NavItemView {
                    label,
                    href: route.href.into(),
                    active: route.is_active(path),
                    external: false,
                });
            }
            NavItemConfig::KIND_PAGE => {
                let Some(page_id) = cfg.page_id else {
                    continue;
                };
                let Some(page) = pages.iter().find(|p| p.id == page_id) else {
                    continue;
                };
                // Page ACL: private pages never appear for guests.
                if !logged_in && page.is_private() {
                    continue;
                }
                let label = if cfg.label.trim().is_empty() {
                    page.title.clone()
                } else {
                    cfg.label.clone()
                };
                let href = page.href();
                let active = path == href;
                out.push(NavItemView {
                    label,
                    href,
                    active,
                    external: false,
                });
            }
            NavItemConfig::KIND_CUSTOM => {
                let Some(url) = sanitize_nav_url(&cfg.url) else {
                    continue;
                };
                let external = url.starts_with("http://") || url.starts_with("https://");
                let active = !external && path == url;
                out.push(NavItemView {
                    label: cfg.label.clone(),
                    href: url,
                    active,
                    external,
                });
            }
            _ => {}
        }
    }

    out
}

pub fn sanitize_nav_url(raw: &str) -> Option<String> {
    let url = raw.trim();
    if url.is_empty() || url.len() > 500 {
        return None;
    }
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        return Some(url.to_string());
    }
    if url.starts_with('/') && !url.starts_with("//") && !lower.starts_with("javascript:") {
        return Some(url.to_string());
    }
    None
}
