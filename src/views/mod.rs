#![allow(dead_code)]

mod admin;
mod auth;
mod docs;
mod public;
mod sparks;

pub use admin::{
    AdminDashboardTemplate, AdminMediaTemplate, AdminNavTemplate, AdminPageEditTemplate,
    AdminPagesTemplate, AdminPostEditTemplate, AdminPostsTemplate, AdminSettingsTemplate,
    AdminTaxonomiesTemplate, AdminTaxonomyEditTemplate,
};
pub use auth::{InstallTemplate, LoginTemplate};
pub use docs::{
    AdminDocsBookTemplate, AdminDocsListTemplate, AdminDocsSettingsTemplate, DocsBookTemplate,
    DocsHomeTemplate,
};
pub use public::{
    HomeTemplate, PageTemplate, PostTemplate, PostsIndexTemplate, SearchTemplate,
    TagsIndexTemplate, TaxonomyPageTemplate,
};
pub use sparks::{
    AdminSparkEditTemplate, AdminSparksTemplate, SparkCaptureTemplate, SparksTemplate,
};
