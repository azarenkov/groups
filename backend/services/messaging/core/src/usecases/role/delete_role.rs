use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, server::id::ServerId, server::unit_of_work::ServerUnitOfWork};
use uuid::Uuid;

use super::{RoleError};

pub struct DeleteRole<U: ServerUnitOfWork> {
    unit_of_work: Arc<U>,
}

impl<U: ServerUnitOfWork> DeleteRole<U> {
    pub fn new(unit_of_work: Arc<U>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        actor: UserId,
        role_id: Uuid,
    ) -> Result<(), RoleError> {
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let mut server = tx.load_server().await?;
        server.delete_role(&actor, role_id, Utc::now())?;
        if !tx.delete_role(role_id).await? {
            return Err(RoleError::RoleNotFound);
        }
        let events: Vec<MessagingEvent> = server.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
