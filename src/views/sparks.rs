use askama::Template;

use crate::infra::pagination::Pagination;
use crate::models::{NavItemView, SiteSettings, Spark};

#[derive(Template)]
#[template(path = "themes/default/sparks.html")]
pub struct SparksTemplate {
    pub settings: SiteSettings,
    pub items: Vec<Spark>,
    pub pagination: Pagination,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
}

#[derive(Template)]
#[template(path = "themes/default/spark_capture.html")]
pub struct SparkCaptureTemplate {
    pub settings: SiteSettings,
    pub error: Option<String>,
    pub content_md: String,
    pub visibility: String,
    pub saved: bool,
}

#[derive(Template)]
#[template(path = "admin/sparks.html")]
pub struct AdminSparksTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub items: Vec<Spark>,
    pub pagination: Pagination,
}

#[derive(Template)]
#[template(path = "admin/spark_edit.html")]
pub struct AdminSparkEditTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub id: i64,
    pub content_md: String,
    pub visibility: String,
    pub error: Option<String>,
}
