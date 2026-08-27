use std::sync::Arc;

use chrono::Utc;
use domain::{user::id::UserId, user::name::ReservedNames, user::name::UserName, user::name::UserNameError};
use ports::{outbox::record::OutboxRecord, outbox::to_outbox::OutboxError, outbox::to_outbox::ToOutboxRecord, repository::RepositoryError};
use identity_domain::{unit_of_work::IdentityUnitOfWork, user::events::UserEvent, user::events::lifecycle::UserRenamed, user::repository::UserRepository};

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

pub struct RenameUser<U, W, TM>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    TM: TextModeration,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    text_moderation: Arc<TM>,
    reserved_names: Arc<ReservedNames>,
}

impl<U, W, TM> RenameUser<U, W, TM>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    TM: TextModeration,
{
    pub fn new(
        user_repository: Arc<U>,
        unit_of_work: Arc<W>,
        text_moderation: Arc<TM>,
        reserved_names: Arc<ReservedNames>,
    ) -> Self {
        Self {
            user_repository,
            unit_of_work,
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

        let persisted = async {
            let mut tx = self.unit_of_work.begin().await?;
            tx.set_user_name(&user_id, &name).await?;
            let event = UserEvent::Renamed(UserRenamed::record(user_id, name.clone(), Utc::now()));
            let records: Vec<OutboxRecord> = vec![event.to_outbox_record()?];
            tx.save_outbox(&records).await?;
            tx.commit().await?;
            Ok::<(), RenameUserError>(())
        }
        .await;
        match persisted {
            Ok(()) => {}
            Err(RenameUserError::Repository(RepositoryError::AlreadyExists)) => {
                return Err(RenameUserError::NameTaken);
            }
            Err(e) => return Err(e),
        }

        Ok(name.as_str().to_string())
    }
}
