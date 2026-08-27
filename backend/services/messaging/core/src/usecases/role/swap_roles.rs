use std::sync::Arc;

use messaging_domain::{server::id::ServerId, server::policy::RolePolicy, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use uuid::Uuid;

use super::{RoleError, roles_changed_event};

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
        let snapshot = tx.authorization().await?;
        RolePolicy::swap(&snapshot, &actor, role_a, role_b)?;
        if !tx.swap_role_positions(role_a, role_b).await? {
            return Err(RoleError::RoleNotFound);
        }
        tx.save_outbox(std::slice::from_ref(&roles_changed_event(&server_id, &snapshot)))
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
