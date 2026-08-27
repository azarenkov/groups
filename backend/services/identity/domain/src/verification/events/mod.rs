use chrono::{DateTime, Utc};

use crate::{user::id::UserId, verification::events::issued::VerificationCodeIssued};

pub mod issued;

#[derive(Debug)]
pub enum VerificationEvent {
    Issued(VerificationCodeIssued),
}

impl VerificationEvent {
    pub fn user_id(&self) -> &UserId {
        match self {
            Self::Issued(e) => &e.user_id,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::Issued(e) => e.occurred_at,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Issued(_) => "verification.issued",
        }
    }
}
