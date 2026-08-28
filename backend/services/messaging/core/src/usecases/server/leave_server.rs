use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;

use messaging_domain::{events::MessagingEvent, server::id::ServerId, server::policy::PolicyError, server::unit_of_work::ServerUnitOfWork};

#[derive(Debug, thiserror::Error)]
pub enum LeaveServerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("You are not a member of this server")]
    NotMember,
    #[error("Server owners cannot leave their own server; delete it instead")]
    OwnerCannotLeave,
}

impl From<PolicyError> for LeaveServerError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => LeaveServerError::NotMember,
            _ => LeaveServerError::OwnerCannotLeave,
        }
    }
}

pub struct LeaveServer<W: ServerUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: ServerUnitOfWork> LeaveServer<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        user_id: UserId,
    ) -> Result<(), LeaveServerError> {
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let mut server = tx.load_server().await?;
        server.leave(&user_id, chrono::Utc::now())?;
        if !tx.remove_member(&user_id).await? {
            return Err(LeaveServerError::NotMember);
        }
        let events: Vec<MessagingEvent> = server.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
