use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::pin::{
    ConversationPinKind, ConversationPinOutcome, ConversationPinRepository, ConversationPins as Pins,
};
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ConversationPinError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Unknown pin kind")]
    InvalidKind,
    #[error("Too many pinned conversations")]
    TooMany,
    #[error("Nothing to pin with this id")]
    UnknownTarget,
    #[error("Cannot pin a conversation with yourself")]
    SelfConversation,
}

pub struct ConversationPins<R: ConversationPinRepository> {
    pins: Arc<R>,
}

impl<R: ConversationPinRepository> ConversationPins<R> {
    pub fn new(pins: Arc<R>) -> Self {
        Self { pins }
    }

    pub async fn list(&self, user: UserId) -> Result<Pins, ConversationPinError> {
        Ok(self.pins.list(&user).await?)
    }

    pub async fn pin(&self, user: UserId, kind: &str, target: Uuid) -> Result<Pins, ConversationPinError> {
        let kind = Self::kind(kind)?;
        if kind == ConversationPinKind::Dm && target == user.uuid() {
            return Err(ConversationPinError::SelfConversation);
        }
        match self.pins.pin(&user, kind, target).await? {
            ConversationPinOutcome::Updated(pins) => Ok(pins),
            ConversationPinOutcome::LimitReached => Err(ConversationPinError::TooMany),
            ConversationPinOutcome::UnknownTarget => Err(ConversationPinError::UnknownTarget),
        }
    }

    pub async fn unpin(&self, user: UserId, kind: &str, target: Uuid) -> Result<Pins, ConversationPinError> {
        let kind = Self::kind(kind)?;
        Ok(self.pins.unpin(&user, kind, target).await?)
    }

    fn kind(kind: &str) -> Result<ConversationPinKind, ConversationPinError> {
        ConversationPinKind::parse(kind).ok_or(ConversationPinError::InvalidKind)
    }
}
