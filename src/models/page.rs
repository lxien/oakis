use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Page {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub content_md: String,
    pub status: String,
    pub visibility: String,
    pub show_in_nav: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl Page {
    pub const VISIBILITY_PRIVATE: &'static str = "private";

    pub fn is_published(&self) -> bool {
        self.status == "published"
    }

    pub fn is_private(&self) -> bool {
        self.visibility == Self::VISIBILITY_PRIVATE
    }

    pub fn updated_display(&self) -> String {
        crate::infra::timefmt::format_local(&self.updated_at)
    }
}
