use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, server::id::ServerId, server::unit_of_work::ServerUnitOfWork};
use uuid::Uuid;

use super::{RoleError};

pub struct SwapRoles<U: ServerUnitOfWork> {
    unit_of_work: Arc<U>,
}

impl<U: ServerUnitOfWork> SwapRoles<U> {
    pub fn new(unit_of_work: Arc<U>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        actor: UserId,
        role_a: Uuid,
        role_b: Uuid,
    ) -> Result<(), RoleError> {
        if role_a == role_b {
            return Err(RoleError::InvalidData);
        }
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let mut server = tx.load_server().await?;
        server.swap_roles(&actor, role_a, role_b, Utc::now())?;
        if !tx.swap_role_positions(role_a, role_b).await? {
            return Err(RoleError::RoleNotFound);
        }
        let events: Vec<MessagingEvent> = server.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
