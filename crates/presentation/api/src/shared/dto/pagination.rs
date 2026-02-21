use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    #[validate(range(min = 1))]
    pub page: i64,
    #[serde(default = "default_per_page")]
    #[validate(range(min = 1, max = 50))]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    10
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

impl PaginationMeta {
    pub fn new(total: i64, page: i64, per_page: i64) -> Self {
        let total_pages = (total as f64 / per_page as f64).ceil() as i64;
        Self {
            total,
            page,
            per_page,
            total_pages,
        }
    }
}
