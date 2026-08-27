use std::sync::Arc;

use messaging_domain::{server::id::ServerId, server::policy::RolePolicy, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use messaging_domain::server::role::RolePatch;
use uuid::Uuid;

use super::validate_role_input;

use super::{RoleError, roles_changed_event};

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
        let snapshot = tx.authorization().await?;
        let existing = snapshot.role(role_id).ok_or(RoleError::RoleNotFound)?;
        validate_role_input(
            patch.name.as_deref().unwrap_or(&existing.name),
            patch.color.as_deref().unwrap_or(&existing.color),
        )?;
        let permissions = RolePolicy::update(&snapshot, &actor, role_id, &patch)?;
        let name = patch.name.as_deref().map(str::trim);
        if !tx
            .update_role(role_id, name, patch.color.as_deref(), permissions)
            .await?
        {
            return Err(RoleError::RoleNotFound);
        }
        tx.save_events(&[roles_changed_event(&server_id, &snapshot)], None)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
