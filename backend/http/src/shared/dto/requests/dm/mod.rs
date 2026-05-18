use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct SendDmMessageBody {
    #[validate(length(min = 1, max = 2000))]
    pub content: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct EditDmMessageBody {
    #[validate(length(min = 1, max = 2000))]
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ListDmMessagesQuery {
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SearchDmMessagesQuery {
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
