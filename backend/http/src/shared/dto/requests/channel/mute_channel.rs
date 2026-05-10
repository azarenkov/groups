use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct MuteRequestBody {
    pub target_user_id: Uuid,
}
