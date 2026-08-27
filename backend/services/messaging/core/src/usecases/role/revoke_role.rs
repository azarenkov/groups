use std::sync::Arc;

use domain::{
    server::{id::ServerId, policy::RolePolicy, unit_of_work::ServerUnitOfWork},
    user::id::UserId,
};
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
        tx.save_outbox(std::slice::from_ref(&roles_changed_event(&server_id, &snapshot)))
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
