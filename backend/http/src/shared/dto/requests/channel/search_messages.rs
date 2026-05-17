use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct SearchMessagesQuery {
    #[validate(length(min = 1, max = 200))]
    pub q: String,
    #[serde(default = "default_search_page")]
    pub page: u32,
    #[serde(default = "default_search_per_page")]
    pub per_page: u32,
}

fn default_search_page() -> u32 {
    1
}

fn default_search_per_page() -> u32 {
    25
}
