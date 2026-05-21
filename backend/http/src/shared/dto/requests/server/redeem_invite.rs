use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RedeemInviteRequestBody {
    #[validate(length(min = 1, max = 16))]
    pub code: String,
}
