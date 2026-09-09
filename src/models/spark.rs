use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Spark {
    pub id: i64,
    pub content_md: String,
    pub content_html: String,
    pub visibility: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Spark {
    pub const VISIBILITY_PRIVATE: &'static str = "private";
    pub const VISIBILITY_PUBLIC: &'static str = "public";

    pub fn is_private(&self) -> bool {
        self.visibility == Self::VISIBILITY_PRIVATE
    }

    pub fn short_date(&self) -> String {
        let full = crate::infra::timefmt::format_local(&self.created_at);
        if full.len() >= 10 {
            full[..10].to_string()
        } else {
            full
        }
    }

    pub fn excerpt(&self) -> String {
        let text = self.content_md.trim();
        if text.is_empty() {
            return String::new();
        }
        let line = text.lines().next().unwrap_or(text).trim();
        const MAX: usize = 80;
        let mut out = String::new();
        for (i, ch) in line.chars().enumerate() {
            if i >= MAX {
                out.push('…');
                break;
            }
            out.push(ch);
        }
        out
    }

    pub fn default_title(&self) -> String {
        let text = self.content_md.trim();
        if text.is_empty() {
            return format!("灵感 {}", self.short_date());
        }
        let line = text.lines().next().unwrap_or(text).trim();
        const MAX: usize = 40;
        let mut out = String::new();
        for (i, ch) in line.chars().enumerate() {
            if i >= MAX {
                out.push('…');
                break;
            }
            out.push(ch);
        }
        if out.is_empty() {
            format!("灵感 {}", self.short_date())
        } else {
            out
        }
    }
}
