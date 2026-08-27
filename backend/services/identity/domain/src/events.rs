use crate::{user::events::UserEvent, verification::events::VerificationEvent};

#[derive(Debug)]
pub enum IdentityEvent {
    User(UserEvent),
    Verification(VerificationEvent),
}

impl From<UserEvent> for IdentityEvent {
    fn from(event: UserEvent) -> Self {
        Self::User(event)
    }
}

impl From<VerificationEvent> for IdentityEvent {
    fn from(event: VerificationEvent) -> Self {
        Self::Verification(event)
    }
}
