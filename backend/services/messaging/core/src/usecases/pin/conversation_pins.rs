use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::{
    community::{id::CommunityId, repository::CommunityRepository},
    pin::{
        ConversationPinKind, ConversationPinRepository, ConversationPins as Pins, ConversationPinsError,
        validate_conversation_pins,
    },
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
    #[error("Duplicate pinned conversation")]
    Duplicate,
    #[error("Cannot pin a conversation with yourself")]
    SelfConversation,
}

impl From<ConversationPinsError> for ConversationPinError {
    fn from(err: ConversationPinsError) -> Self {
        match err {
            ConversationPinsError::TooMany => Self::TooMany,
            ConversationPinsError::Duplicate => Self::Duplicate,
        }
    }
}

pub struct ConversationPins<R: ConversationPinRepository, S: CommunityRepository> {
    pins: Arc<R>,
    communities: Arc<S>,
}

impl<R: ConversationPinRepository, S: CommunityRepository> ConversationPins<R, S> {
    pub fn new(pins: Arc<R>, communities: Arc<S>) -> Self {
        Self { pins, communities }
    }

    pub async fn list(&self, user: UserId) -> Result<Pins, ConversationPinError> {
        Ok(self.pins.list(&user).await?)
    }

    pub async fn replace(&self, user: UserId, kind: &str, ids: Vec<Uuid>) -> Result<Pins, ConversationPinError> {
        let kind = ConversationPinKind::parse(kind).ok_or(ConversationPinError::InvalidKind)?;
        validate_conversation_pins(&ids)?;
        let ids = match kind {
            ConversationPinKind::Dm => {
                if ids.contains(&user.uuid()) {
                    return Err(ConversationPinError::SelfConversation);
                }
                ids
            }
            ConversationPinKind::Community => {
                let mut kept = Vec::with_capacity(ids.len());
                for id in ids {
                    if self.communities.is_community_member(&CommunityId::new(id), &user).await? {
                        kept.push(id);
                    }
                }
                kept
            }
        };
        Ok(self.pins.replace(&user, kind, &ids).await?)
    }
}
