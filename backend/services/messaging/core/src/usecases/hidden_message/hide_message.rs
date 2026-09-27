use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::hidden_message::repository::HiddenMessageRepository;
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum HideMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct HideMessage<H: HiddenMessageRepository> {
    repo: Arc<H>,
}

impl<H: HiddenMessageRepository> HideMessage<H> {
    pub fn new(repo: Arc<H>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user_id: UserId, message_id: Uuid) -> Result<(), HideMessageError> {
        self.repo.hide_for_user(&user_id, message_id).await?;
        Ok(())
    }
}
