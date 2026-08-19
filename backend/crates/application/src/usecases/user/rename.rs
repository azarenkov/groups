use std::sync::Arc;

use chrono::Utc;
use domain::{
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::{
        events::{UserEvent, lifecycle::UserRenamed},
        id::UserId,
        name::{ReservedNames, UserName, UserNameError},
        repository::UserRepository,
    },
};

use crate::ports::text_moderation::{TextModeration, TextModerationError};

#[derive(Debug, thiserror::Error)]
pub enum RenameUserError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Invalid username: {0}")]
    InvalidName(#[from] UserNameError),
    #[error("Username is already taken")]
    NameTaken,
    #[error("Username is not allowed")]
    NameNotAllowed,
    #[error("Username moderation unavailable: {0}")]
    ModerationUnavailable(String),
}

pub struct RenameUser<U, F, TM>
where
    U: UserRepository,
    F: FriendshipRepository,
    TM: TextModeration,
{
    user_repository: Arc<U>,
    friendship_repository: Arc<F>,
    text_moderation: Arc<TM>,
    reserved_names: Arc<ReservedNames>,
}

impl<U, F, TM> RenameUser<U, F, TM>
where
    U: UserRepository,
    F: FriendshipRepository,
    TM: TextModeration,
{
    pub fn new(
        user_repository: Arc<U>,
        friendship_repository: Arc<F>,
        text_moderation: Arc<TM>,
        reserved_names: Arc<ReservedNames>,
    ) -> Self {
        Self {
            user_repository,
            friendship_repository,
            text_moderation,
            reserved_names,
        }
    }

    pub async fn execute(&self, user_id: UserId, name: &str) -> Result<String, RenameUserError> {
        let name = UserName::parse(name, &self.reserved_names)?;

        if let Some(existing) = self.user_repository.find_by_name(&name).await?
            && existing.id_ownned() != user_id
        {
            return Err(RenameUserError::NameTaken);
        }

        match self.text_moderation.ensure_allowed(name.as_str()).await {
            Ok(()) => {}
            Err(TextModerationError::Rejected) => {
                return Err(RenameUserError::NameNotAllowed);
            }
            Err(TextModerationError::Unavailable(e)) => {
                return Err(RenameUserError::ModerationUnavailable(e));
            }
        }

        let friends = self
            .friendship_repository
            .list_for_user(&user_id, None)
            .await?;
        let mut recipients: Vec<UserId> = friends
            .iter()
            .filter(|f| f.status != FriendshipStatus::Declined)
            .map(|f| f.friend_id)
            .collect();
        recipients.push(user_id);

        let event = UserEvent::Renamed(UserRenamed::record(
            user_id,
            name.clone(),
            recipients,
            Utc::now(),
        ));
        let records: Vec<OutboxRecord> = vec![event.to_outbox_record()?];

        match self
            .user_repository
            .set_name_with_events(&user_id, &name, &records)
            .await
        {
            Ok(()) => {}
            Err(RepositoryError::AlreadyExists) => {
                return Err(RenameUserError::NameTaken);
            }
            Err(e) => return Err(e.into()),
        }

        Ok(name.as_str().to_string())
    }
}
