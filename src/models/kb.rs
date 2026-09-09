use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct KbBook {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub cover_url: String,
    pub status: String,
    pub visibility: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl KbBook {
    pub fn is_published(&self) -> bool {
        self.status == "published"
    }

    pub fn is_private(&self) -> bool {
        self.visibility == "private"
    }

    pub fn is_publicly_visible(&self) -> bool {
        self.is_published() && !self.is_private()
    }

    pub fn has_cover(&self) -> bool {
        !self.cover_url.is_empty()
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct KbNode {
    pub id: i64,
    pub book_id: i64,
    pub parent_id: Option<i64>,
    pub node_type: String,
    pub title: String,
    pub slug: String,
    pub sort_order: i64,
    pub content_md: String,
    pub content_html: String,
    pub status: String,
    pub visibility: String,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl KbNode {
    pub const TYPE_FOLDER: &'static str = "folder";
    pub const TYPE_DOC: &'static str = "doc";

    pub fn is_folder(&self) -> bool {
        self.node_type == Self::TYPE_FOLDER
    }

    pub fn is_doc(&self) -> bool {
        self.node_type == Self::TYPE_DOC
    }

    pub fn is_published(&self) -> bool {
        self.status == "published"
    }

    pub fn is_private(&self) -> bool {
        self.visibility == "private"
    }

    pub fn is_publicly_visible(&self) -> bool {
        self.is_doc() && self.is_published() && !self.is_private()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KbTreeNode {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub node_type: String,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub visibility: String,
    pub updated_at: String,
    pub children: Vec<KbTreeNode>,
}

impl KbTreeNode {
    pub fn is_folder(&self) -> bool {
        self.node_type == KbNode::TYPE_FOLDER
    }

    pub fn is_doc(&self) -> bool {
        self.node_type == KbNode::TYPE_DOC
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn is_draft(&self) -> bool {
        self.status == "draft"
    }

    pub fn is_private(&self) -> bool {
        self.visibility == "private"
    }
}

#[derive(Debug, Clone)]
pub struct KbCrumb {
    pub title: String,
    pub href: Option<String>,
}

impl KbCrumb {
    pub fn has_href(&self) -> bool {
        self.href.is_some()
    }

    pub fn href_value(&self) -> &str {
        self.href.as_deref().unwrap_or("#")
    }
}

#[derive(Debug, Clone)]
pub struct KbNeighbor {
    pub title: String,
    pub href: String,
}
