use serde::Deserialize;

pub const ADMIN_PER_PAGE: u32 = 10;

pub const PUBLIC_PER_PAGE: u32 = 100;

pub const PUBLIC_TAX_PER_PAGE: u32 = 50;

#[derive(Debug, Clone, Deserialize)]
pub struct PageQuery {
    pub page: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Pagination {
    pub page: u32,
    pub per_page: u32,
    pub total: i64,

    pub base_url: String,
}

impl Pagination {
    pub fn new(page: Option<u32>, per_page: u32, total: i64, base_url: impl Into<String>) -> Self {
        let mut page = page.unwrap_or(1).max(1);
        let total_pages = Self::calc_total_pages(total, per_page);
        if total_pages > 0 && page > total_pages {
            page = total_pages;
        }
        Self {
            page,
            per_page,
            total,
            base_url: base_url.into(),
        }
    }

    fn calc_total_pages(total: i64, per_page: u32) -> u32 {
        if total <= 0 || per_page == 0 {
            return 0;
        }
        ((total as u64 + per_page as u64 - 1) / per_page as u64) as u32
    }

    pub fn offset(&self) -> i64 {
        ((self.page as i64) - 1) * (self.per_page as i64)
    }

    pub fn limit(&self) -> i64 {
        self.per_page as i64
    }

    pub fn total_pages(&self) -> u32 {
        Self::calc_total_pages(self.total, self.per_page)
    }

    pub fn has_next(&self) -> bool {
        let tp = self.total_pages();
        tp > 0 && self.page < tp
    }

    pub fn next_page(&self) -> u32 {
        let tp = self.total_pages();
        if tp == 0 { 1 } else { (self.page + 1).min(tp) }
    }

    pub fn show(&self) -> bool {
        self.total > self.per_page as i64
    }

    pub fn url_for(&self, page: u32) -> String {
        let sep = if self.base_url.contains('?') {
            '&'
        } else {
            '?'
        };
        format!("{}{}page={}", self.base_url, sep, page)
    }

    pub fn next_url(&self) -> String {
        self.url_for(self.next_page())
    }
}
