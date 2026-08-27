use std::sync::Arc;

use domain::{
    server::{id::ServerId, policy::RolePolicy, unit_of_work::ServerUnitOfWork},
    user::id::UserId,
};
use domain::server::role::RolePatch;
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
        tx.save_outbox(std::slice::from_ref(&roles_changed_event(&server_id, &snapshot)))
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
