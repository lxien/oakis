use askama::Template;

use crate::infra::pagination::Pagination;
use crate::models::{
    KbBook, NavItemView, Page, PostView, SearchHit, SiteSettings, Taxonomy, TaxonomyCount,
};

#[derive(Template)]
#[template(path = "themes/default/home.html")]
pub struct HomeTemplate {
    pub settings: SiteSettings,
    pub posts: Vec<PostView>,
    pub featured: Option<PostView>,
    pub categories: Vec<TaxonomyCount>,
    pub categories_more: bool,
    pub tags: Vec<TaxonomyCount>,
    pub tags_more: bool,
    pub books: Vec<KbBook>,
    pub books_more: bool,
    pub pagination: Pagination,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
}

#[derive(Template)]
#[template(path = "themes/default/posts.html")]
pub struct PostsIndexTemplate {
    pub settings: SiteSettings,
    pub posts: Vec<PostView>,
    pub categories: Vec<TaxonomyCount>,
    pub uncategorized_count: i64,

    pub active_filter: String,
    pub pagination: Pagination,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
}

#[derive(Template)]
#[template(path = "themes/default/post.html")]
pub struct PostTemplate {
    pub settings: SiteSettings,
    pub post: PostView,
    pub related: Vec<crate::models::RelatedPostRef>,
    pub toc: Vec<crate::infra::markdown::TocItem>,
    pub categories: Vec<TaxonomyCount>,
    pub categories_more: bool,
    pub tags: Vec<TaxonomyCount>,
    pub tags_more: bool,
    pub books: Vec<KbBook>,
    pub books_more: bool,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
}

#[derive(Template)]
#[template(path = "themes/default/page.html")]
pub struct PageTemplate {
    pub settings: SiteSettings,
    pub page: Page,
    pub content_html: String,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
}

#[derive(Template)]
#[template(path = "themes/default/search.html")]
pub struct SearchTemplate {
    pub settings: SiteSettings,
    pub query: String,
    pub hits: Vec<SearchHit>,
    pub pagination: Pagination,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
}

#[derive(Template)]
#[template(path = "themes/default/taxonomy.html")]
pub struct TaxonomyPageTemplate {
    pub settings: SiteSettings,
    pub term: Taxonomy,
    pub kind_label: String,
    pub posts: Vec<PostView>,
    pub pagination: Pagination,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
}

#[derive(Template)]
#[template(path = "themes/default/tags.html")]
pub struct TagsIndexTemplate {
    pub settings: SiteSettings,
    pub tags: Vec<TaxonomyCount>,
    pub logged_in: bool,
    pub nav_items: Vec<NavItemView>,
    pub search_query: String,
}
