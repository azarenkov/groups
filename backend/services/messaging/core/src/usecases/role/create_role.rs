use std::sync::Arc;

use messaging_domain::{server::id::ServerId, server::policy::RolePolicy, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use chrono::Utc;
use messaging_domain::server::role::{PERM_ALL, ServerRole};
use uuid::Uuid;

use super::validate_role_input;

use super::{RoleError, roles_changed_event};

pub struct CreateRole<U: ServerUnitOfWork> {
    unit_of_work: Arc<U>,
}

impl<U: ServerUnitOfWork> CreateRole<U> {
    pub fn new(unit_of_work: Arc<U>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        actor: UserId,
        name: String,
        color: String,
        permissions: i64,
    ) -> Result<ServerRole, RoleError> {
        validate_role_input(&name, &color)?;
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let snapshot = tx.authorization().await?;
        let position = RolePolicy::create(&snapshot, &actor, permissions)?;
        let role = ServerRole {
            id: Uuid::new_v4(),
            server_id: server_id.uuid(),
            name: name.trim().to_string(),
            color,
            permissions: permissions & PERM_ALL,
            position,
            created_at: Utc::now(),
        };
        tx.insert_role(&role).await?;
        tx.save_events(&[roles_changed_event(&server_id, &snapshot)], None)
            .await?;
        tx.commit().await?;
        Ok(role)
    }
}
