use askama::Template;

use crate::models::{KbBook, KbCrumb, KbNeighbor, NavItemView, SiteSettings};

#[derive(Template)]
#[template(path = "themes/default/docs_home.html")]
pub struct DocsHomeTemplate {
    pub settings: SiteSettings,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
    pub books: Vec<KbBook>,
    pub can_manage: bool,
}

impl DocsHomeTemplate {
    pub fn is_empty(&self) -> bool {
        self.books.is_empty()
    }
}

#[derive(Template)]
#[template(path = "themes/default/docs.html")]
pub struct DocsBookTemplate {
    pub settings: SiteSettings,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
    pub book: KbBook,
    pub tree_html: String,
    pub title: String,
    pub content_html: String,
    pub crumbs: Vec<KbCrumb>,
    pub prev: Option<KbNeighbor>,
    pub next: Option<KbNeighbor>,
    pub empty: bool,
    pub is_home: bool,
    pub catalog_html: String,
    pub doc_count: usize,
    pub word_count: usize,
    pub edit_href: Option<String>,
}

impl DocsBookTemplate {
    pub fn has_crumbs(&self) -> bool {
        !self.crumbs.is_empty()
    }

    pub fn has_nav(&self) -> bool {
        self.prev.is_some() || self.next.is_some()
    }

    pub fn edit_url(&self) -> &str {
        self.edit_href.as_deref().unwrap_or("")
    }

    pub fn can_edit(&self) -> bool {
        self.edit_href.is_some()
    }

    pub fn has_catalog(&self) -> bool {
        !self.catalog_html.is_empty()
    }

    pub fn has_summary(&self) -> bool {
        !self.content_html.is_empty()
    }
}

#[derive(Template)]
#[template(path = "admin/docs_list.html")]
pub struct AdminDocsListTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub books: Vec<KbBook>,
    pub error: Option<String>,
}

impl AdminDocsListTemplate {
    pub fn is_empty(&self) -> bool {
        self.books.is_empty()
    }
}

#[derive(Template)]
#[template(path = "admin/docs_settings.html")]
pub struct AdminDocsSettingsTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub book: KbBook,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/docs.html")]
pub struct AdminDocsBookTemplate {
    pub settings: SiteSettings,
    pub username: String,
    pub book: KbBook,
    pub tree_html: String,
    pub has_nodes: bool,
    pub selected_id: Option<i64>,
    pub is_folder: bool,
    pub title: String,
    pub slug: String,
    pub content_md: String,
    pub status: String,
    pub visibility: String,

    pub published_at_local: String,
    pub form_action: String,
    pub catalog_html: String,
    pub doc_count: usize,
    pub word_count: usize,
    pub error: Option<String>,
}

impl AdminDocsBookTemplate {
    pub fn is_home(&self) -> bool {
        self.selected_id.is_none()
    }

    pub fn has_catalog(&self) -> bool {
        !self.catalog_html.is_empty()
    }

    pub fn selected_id_value(&self) -> i64 {
        self.selected_id.unwrap_or(0)
    }

    pub fn preview_href(&self) -> String {
        format!("/docs/{}/{}", self.book.slug, self.slug)
    }

    pub fn show_doc_editor(&self) -> bool {
        self.selected_id.is_some() && !self.is_folder
    }

    pub fn show_folder_panel(&self) -> bool {
        self.selected_id.is_some() && self.is_folder
    }
}
