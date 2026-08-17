use std::sync::Arc;

use domain::{
    shared::repository::error::RepositoryError,
    user::{
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
    #[error("Invalid username: {0}")]
    InvalidName(#[from] UserNameError),
    #[error("Username is already taken")]
    NameTaken,
    #[error("Username is not allowed")]
    NameNotAllowed,
    #[error("Username moderation unavailable: {0}")]
    ModerationUnavailable(String),
}

pub struct RenameUser<U, TM>
where
    U: UserRepository,
    TM: TextModeration,
{
    user_repository: Arc<U>,
    text_moderation: Arc<TM>,
    reserved_names: Arc<ReservedNames>,
}

impl<U, TM> RenameUser<U, TM>
where
    U: UserRepository,
    TM: TextModeration,
{
    pub fn new(
        user_repository: Arc<U>,
        text_moderation: Arc<TM>,
        reserved_names: Arc<ReservedNames>,
    ) -> Self {
        Self {
            user_repository,
            text_moderation,
            reserved_names,
        }
    }

    pub async fn execute(&self, user_id: UserId, name: &str) -> Result<String, RenameUserError> {
        let name = UserName::parse(name, &self.reserved_names)?;

        if let Some(existing) = self.user_repository.find_by_name(&name).await? {
            if existing.id_ownned() != user_id {
                return Err(RenameUserError::NameTaken);
            }
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

        match self.user_repository.set_name(&user_id, &name).await {
            Ok(()) => {}
            Err(RepositoryError::AlreadyExists) => {
                return Err(RenameUserError::NameTaken);
            }
            Err(e) => return Err(e.into()),
        }

        Ok(name.as_str().to_string())
    }
}
