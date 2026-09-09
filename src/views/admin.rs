use askama::Template;

use crate::infra::pagination::Pagination;
use crate::models::{Media, NavItemConfig, NavPageRef, Page, SiteSettings, Taxonomy};
use crate::web::TaxOption;

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct AdminDashboardTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub dash: crate::store::DashboardData,
}

#[derive(Template)]
#[template(path = "admin/posts.html")]
pub struct AdminPostsTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub posts: Vec<crate::models::Post>,
    pub pagination: Pagination,
}

#[derive(Template)]
#[template(path = "admin/post_edit.html")]
pub struct AdminPostEditTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub is_new: bool,
    pub content_id: i64,
    pub form_action: String,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub content_md: String,
    pub cover_url: String,
    pub status: String,
    pub visibility: String,

    pub published_at_local: String,
    pub categories: Vec<TaxOption>,
    pub tags: Vec<TaxOption>,
    pub related: Vec<crate::models::RelatedPostRef>,
    pub related_max: usize,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/settings.html")]
pub struct AdminSettingsTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub error: Option<String>,
    pub saved: bool,
}

#[derive(Template)]
#[template(path = "admin/nav.html")]
pub struct AdminNavTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub items: Vec<NavItemConfig>,
    pub pages: Vec<NavPageRef>,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/taxonomies.html")]
pub struct AdminTaxonomiesTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub kind: String,
    pub kind_label: String,
    pub items: Vec<Taxonomy>,
    pub pagination: Pagination,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/taxonomy_edit.html")]
pub struct AdminTaxonomyEditTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub id: i64,
    pub kind: String,
    pub kind_label: String,
    pub name: String,
    pub slug: String,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/media.html")]
pub struct AdminMediaTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub items: Vec<Media>,
    pub pagination: Pagination,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/pages.html")]
pub struct AdminPagesTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub pages: Vec<Page>,
    pub pagination: Pagination,
}

#[derive(Template)]
#[template(path = "admin/page_edit.html")]
pub struct AdminPageEditTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub is_new: bool,
    pub content_id: i64,
    pub form_action: String,
    pub title: String,
    pub slug: String,
    pub content_md: String,
    pub status: String,
    pub visibility: String,
    pub error: Option<String>,
}
