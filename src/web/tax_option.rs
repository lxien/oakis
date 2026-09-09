use std::collections::HashSet;

use crate::models::Taxonomy;

#[derive(Clone)]
pub struct TaxOption {
    pub id: i64,
    pub name: String,
    pub selected: bool,
}

impl TaxOption {
    pub fn from_list(all: Vec<Taxonomy>, selected: &HashSet<i64>) -> Vec<Self> {
        all.into_iter()
            .map(|t| Self {
                id: t.id,
                name: t.name,
                selected: selected.contains(&t.id),
            })
            .collect()
    }
}
