use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    shared::event::event_id::EventId,
    user::{email::Email, id::UserId, locale::Locale},
    verification::{
        channel::VerificationChannel, id::VerificationCodeId, purpose::VerificationPurpose,
    },
};

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationCodeIssued {
    pub id: EventId,
    pub user_id: UserId,
    pub verification_code_id: VerificationCodeId,
    pub channel: VerificationChannel,
    pub purpose: VerificationPurpose,
    pub email: Email,
    pub code: String,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub locale: Locale,
}

impl VerificationCodeIssued {
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        user_id: UserId,
        verification_code_id: VerificationCodeId,
        channel: VerificationChannel,
        purpose: VerificationPurpose,
        email: Email,
        code: String,
        now: DateTime<Utc>,
        locale: Locale,
    ) -> Self {
        Self {
            id: EventId::generate(),
            user_id,
            verification_code_id,
            channel,
            purpose,
            email,
            code,
            occurred_at: now,
            locale,
        }
    }
}
