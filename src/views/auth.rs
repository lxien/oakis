use askama::Template;

use crate::models::SiteSettings;

#[derive(Template)]
#[template(path = "themes/default/install.html")]
pub struct InstallTemplate {
    pub error: Option<String>,
    pub site_title: String,
    pub site_tagline: String,
    pub author_name: String,
    pub username: String,
}

#[derive(Template)]
#[template(path = "admin/login.html")]
pub struct LoginTemplate {
    pub settings: SiteSettings,
    pub login_path: String,
    pub error: Option<String>,
    pub username: String,
    pub next: String,
}
