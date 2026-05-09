use chrono::{DateTime, Duration, Utc};
use rand::RngExt;

use crate::{
    user::value_objects::id::UserId,
    verification::{
        channel::VerificationChannel, code_hash::CodeHash, id::VerificationCodeId,
        purpose::VerificationPurpose,
    },
};

#[derive(Debug)]
pub struct VerificationCode {
    id: VerificationCodeId,
    user_id: UserId,
    channel: VerificationChannel,
    code_hash: CodeHash,
    purpose: VerificationPurpose,
    expires_at: DateTime<Utc>,
    consumed_at: Option<DateTime<Utc>>,
    attempts: u8,
    created_at: DateTime<Utc>,
}

impl VerificationCode {
    pub fn create(
        user_id: UserId,
        channel: VerificationChannel,
        code_hash: CodeHash,
        purpose: VerificationPurpose,
    ) -> Self {
        let now = Utc::now();
        let ttl = Self::ttl_for(&purpose);

        Self {
            id: VerificationCodeId::generate(),
            user_id,
            channel,
            code_hash,
            purpose,
            expires_at: now + ttl,
            consumed_at: None,
            attempts: 0,
            created_at: now,
        }
    }

    fn ttl_for(purpose: &VerificationPurpose) -> Duration {
        match purpose {
            VerificationPurpose::Registration => Duration::minutes(30),
            VerificationPurpose::PasswordReset => Duration::minutes(10),
            VerificationPurpose::EmailChange => Duration::minutes(10),
            VerificationPurpose::PhoneChange => Duration::minutes(10),
        }
    }

    pub fn generate() -> String {
        let code = rand::rng().random_range(0..1_000_000);
        format!("{:06}", code)
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn channel(&self) -> &VerificationChannel {
        &self.channel
    }

    pub fn code_hash(&self) -> &CodeHash {
        &self.code_hash
    }

    pub fn purpose(&self) -> &VerificationPurpose {
        &self.purpose
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub fn consumed_at(&self) -> Option<DateTime<Utc>> {
        self.consumed_at
    }

    pub fn attempts(&self) -> u8 {
        self.attempts
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn id(&self) -> &VerificationCodeId {
        &self.id
    }
}
