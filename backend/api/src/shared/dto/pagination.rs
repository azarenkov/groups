use domain::shared::pagination::{
    DEFAULT_PAGE, DEFAULT_PER_PAGE, MAX_PER_PAGE, MIN_PAGES, MIN_PER_PAGE,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    #[validate(range(min = MIN_PAGES))]
    pub page: u32,
    #[serde(default = "default_per_page")]
    #[validate(range(min = MIN_PER_PAGE, max = MAX_PER_PAGE))]
    pub per_page: u32,
}

fn default_page() -> u32 {
    DEFAULT_PAGE
}

fn default_per_page() -> u32 {
    DEFAULT_PER_PAGE
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: i64,
}

impl PaginationMeta {
    pub fn new(total: i64, page: u32, per_page: u32) -> Self {
        let total_pages = if per_page == 0 || total <= 0 {
            0
        } else {
            let per_page = per_page as i64;
            (total + per_page - 1) / per_page
        };

        Self {
            total,
            page,
            per_page,
            total_pages,
        }
    }
}
