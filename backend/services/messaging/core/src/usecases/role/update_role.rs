use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, server::id::ServerId, server::unit_of_work::ServerUnitOfWork, server::role::RolePatch};
use uuid::Uuid;

use super::{RoleError, validate_role_input};

pub struct UpdateRole<U: ServerUnitOfWork> {
    unit_of_work: Arc<U>,
}

impl<U: ServerUnitOfWork> UpdateRole<U> {
    pub fn new(unit_of_work: Arc<U>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        actor: UserId,
        role_id: Uuid,
        patch: RolePatch,
    ) -> Result<(), RoleError> {
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let mut server = tx.load_server().await?;
        let existing = server.role(role_id).ok_or(RoleError::RoleNotFound)?;
        validate_role_input(
            patch.name.as_deref().unwrap_or(&existing.name),
            patch.color.as_deref().unwrap_or(&existing.color),
        )?;
        let updated = server.update_role(&actor, role_id, &patch, Utc::now())?;
        if !tx
            .update_role(role_id, Some(&updated.name), Some(&updated.color), updated.permissions)
            .await?
        {
            return Err(RoleError::RoleNotFound);
        }
        let events: Vec<MessagingEvent> = server.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
