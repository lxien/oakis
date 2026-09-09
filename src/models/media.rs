use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Media {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub mime: String,
    pub size: i64,
    pub created_at: String,
}

impl Media {
    pub fn is_image(&self) -> bool {
        self.mime.starts_with("image/")
    }

    pub fn thumb_url(&self) -> String {
        let t = crate::infra::upload::media_thumb_url(&self.url);
        if t.is_empty() { self.url.clone() } else { t }
    }

    pub fn created_display(&self) -> String {
        crate::infra::timefmt::format_local(&self.created_at)
    }

    pub fn size_label(&self) -> String {
        let n = self.size as f64;
        if n < 1024.0 {
            format!("{} B", self.size)
        } else if n < 1024.0 * 1024.0 {
            format!("{:.1} KB", n / 1024.0)
        } else {
            format!("{:.1} MB", n / (1024.0 * 1024.0))
        }
    }
}
