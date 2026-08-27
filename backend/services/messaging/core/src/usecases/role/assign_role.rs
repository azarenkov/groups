use std::sync::Arc;

use messaging_domain::{server::id::ServerId, server::policy::RolePolicy, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use uuid::Uuid;

use super::{RoleError, roles_changed_event};

pub struct AssignRole<U: ServerUnitOfWork> {
    unit_of_work: Arc<U>,
}

impl<U: ServerUnitOfWork> AssignRole<U> {
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
        RolePolicy::assign(&snapshot, &actor, &target, role_id)?;
        tx.assign_role(&target, role_id).await?;
        tx.save_outbox(std::slice::from_ref(&roles_changed_event(&server_id, &snapshot)))
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
