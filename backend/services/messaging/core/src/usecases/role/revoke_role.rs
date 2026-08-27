use std::sync::Arc;

use messaging_domain::{server::id::ServerId, server::policy::RolePolicy, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use uuid::Uuid;

use super::{RoleError, roles_changed_event};

pub struct RevokeRole<U: ServerUnitOfWork> {
    unit_of_work: Arc<U>,
}

impl<U: ServerUnitOfWork> RevokeRole<U> {
    pub fn new(unit_of_work: Arc<U>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        actor: UserId,
        target: UserId,
        role_id: Uuid,
    ) -> Result<(), RoleError> {
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let snapshot = tx.authorization().await?;
        RolePolicy::revoke(&snapshot, &actor, &target, role_id)?;
        if !tx.revoke_role(&target, role_id).await? {
            return Err(RoleError::RoleNotFound);
        }
        tx.save_events(&[roles_changed_event(&server_id, &snapshot)], None)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
