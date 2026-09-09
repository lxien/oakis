use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::Taxonomy;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub content_md: String,
    pub content_html: String,
    pub status: String,
    pub visibility: String,
    pub cover_url: Option<String>,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct RelatedPostRef {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub published_at: Option<String>,
    pub created_at: String,
}

impl RelatedPostRef {
    pub const TARGET: usize = 5;

    pub const MANUAL_MAX: usize = 8;

    pub fn short_date(&self) -> &str {
        let d = self
            .published_at
            .as_deref()
            .unwrap_or(self.created_at.as_str());
        if d.len() >= 10 { &d[..10] } else { d }
    }

    pub fn display_date(&self) -> &str {
        self.published_at
            .as_deref()
            .unwrap_or(self.created_at.as_str())
    }

    pub fn from_post(post: &Post) -> Self {
        Self {
            id: post.id,
            title: post.title.clone(),
            slug: post.slug.clone(),
            published_at: post.published_at.clone(),
            created_at: post.created_at.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PostView {
    pub post: Post,
    pub categories: Vec<Taxonomy>,
    pub tags: Vec<Taxonomy>,
}

impl PostView {
    pub fn title(&self) -> &str {
        &self.post.title
    }

    pub fn slug(&self) -> &str {
        &self.post.slug
    }

    pub fn summary(&self) -> &str {
        &self.post.summary
    }

    pub fn short_date(&self) -> &str {
        self.post.short_date()
    }

    pub fn display_date(&self) -> &str {
        self.post.display_date()
    }

    pub fn has_cover(&self) -> bool {
        self.post.has_cover()
    }

    pub fn cover(&self) -> &str {
        self.post.cover()
    }

    pub fn reading_minutes(&self) -> u32 {
        self.post.reading_minutes()
    }

    pub fn id(&self) -> i64 {
        self.post.id
    }

    pub fn content_html(&self) -> &str {
        &self.post.content_html
    }

    pub fn has_categories(&self) -> bool {
        !self.categories.is_empty()
    }

    pub fn has_tags(&self) -> bool {
        !self.tags.is_empty()
    }

    pub fn is_private(&self) -> bool {
        self.post.is_private()
    }
}

impl Post {
    pub const VISIBILITY_PRIVATE: &'static str = "private";

    pub fn is_published(&self) -> bool {
        self.status == "published"
    }

    pub fn is_private(&self) -> bool {
        self.visibility == Self::VISIBILITY_PRIVATE
    }

    pub fn display_date(&self) -> &str {
        self.published_at
            .as_deref()
            .unwrap_or(self.created_at.as_str())
    }

    pub fn short_date(&self) -> &str {
        let d = self.display_date();
        if d.len() >= 10 { &d[..10] } else { d }
    }

    pub fn updated_display(&self) -> String {
        crate::infra::timefmt::format_local(&self.updated_at)
    }

    pub fn sanitize_cover(&mut self) {
        if let Some(url) = self.cover_url.take() {
            let safe = crate::infra::upload::sanitize_cover_url(&url);
            self.cover_url = if safe.is_empty() { None } else { Some(safe) };
        }
    }

    pub fn has_cover(&self) -> bool {
        self.cover_url.as_deref().is_some_and(|u| !u.is_empty())
    }

    pub fn cover(&self) -> &str {
        self.cover_url.as_deref().unwrap_or("")
    }

    pub fn reading_minutes(&self) -> u32 {
        let chars = self
            .content_md
            .chars()
            .filter(|c| !c.is_whitespace())
            .count();
        ((chars as u32).div_ceil(400)).max(1)
    }
}
