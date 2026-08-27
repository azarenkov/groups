use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;

use messaging_domain::{events::MessagingEvent, server::id::ServerId, server::policy::PolicyError, server::unit_of_work::ServerUnitOfWork};

#[derive(Debug, thiserror::Error)]
pub enum KickMemberError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Target is not a member of this server")]
    NotMember,
    #[error("Missing permission to kick members")]
    NotAllowed,
}

impl From<PolicyError> for KickMemberError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::TargetNotMember => KickMemberError::NotMember,
            _ => KickMemberError::NotAllowed,
        }
    }
}

pub struct KickMember<W: ServerUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: ServerUnitOfWork> KickMember<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        actor: UserId,
        target: UserId,
    ) -> Result<(), KickMemberError> {
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let mut server = tx.load_server().await?;
        server.kick_member(&actor, &target, chrono::Utc::now())?;
        if !tx.remove_member(&target).await? {
            return Err(KickMemberError::NotMember);
        }
        let events: Vec<MessagingEvent> = server.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
