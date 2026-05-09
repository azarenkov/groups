use chrono::{DateTime, Utc};
use sqlx::{FromRow, types::Uuid};

use super::{
    verification_channel::VerificationChannelDto, verification_purpose::VerificationPurposeDto,
};

#[derive(Debug, FromRow)]
pub struct VerificationCodeDto {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel: VerificationChannelDto,
    pub code_hash: String,
    pub purpose: VerificationPurposeDto,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub attempts: i16,
    pub created_at: DateTime<Utc>,
}
