use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Taxonomy {
    pub id: i64,
    pub scope: String,
    pub kind: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct TaxonomyCount {
    pub name: String,
    pub slug: String,
    pub post_count: i64,
}

impl Taxonomy {
    pub const SCOPE_POST: &'static str = "post";
    pub const KIND_CATEGORY: &'static str = "category";
    pub const KIND_TAG: &'static str = "tag";

    pub fn is_category(&self) -> bool {
        self.kind == Self::KIND_CATEGORY
    }

    pub fn is_tag(&self) -> bool {
        self.kind == Self::KIND_TAG
    }
}

pub const ASIDE_TAX_LIMIT: usize = 100;

pub fn take_aside_tax(mut items: Vec<TaxonomyCount>) -> (Vec<TaxonomyCount>, bool) {
    let more = items.len() > ASIDE_TAX_LIMIT;
    if more {
        items.truncate(ASIDE_TAX_LIMIT);
    }
    (items, more)
}

pub fn take_aside_list<T>(mut items: Vec<T>) -> (Vec<T>, bool) {
    let more = items.len() > ASIDE_TAX_LIMIT;
    if more {
        items.truncate(ASIDE_TAX_LIMIT);
    }
    (items, more)
}
