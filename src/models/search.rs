use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SearchHit {
    pub kind: String,
    pub title: String,
    pub href: String,
    pub excerpt: String,
    pub sort_at: String,
}

impl SearchHit {
    pub fn kind_label(&self) -> &'static str {
        match self.kind.as_str() {
            "post" => "文章",
            "page" => "页面",
            "doc" => "知识库",
            _ => "内容",
        }
    }

    pub fn short_date(&self) -> &str {
        self.sort_at.get(..10).unwrap_or(&self.sort_at)
    }

    pub fn display_excerpt(&self) -> String {
        let flat: String = self
            .excerpt
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .collect();
        let flat = flat.split_whitespace().collect::<Vec<_>>().join(" ");
        const MAX: usize = 140;
        let mut out = String::new();
        for (i, ch) in flat.chars().enumerate() {
            if i >= MAX {
                out.push('…');
                break;
            }
            out.push(ch);
        }
        out
    }

    pub fn has_excerpt(&self) -> bool {
        !self.display_excerpt().is_empty()
    }
}
