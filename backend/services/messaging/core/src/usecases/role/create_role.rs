use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, server::id::ServerId, server::unit_of_work::ServerUnitOfWork, server::role::ServerRole};

use super::{RoleError, validate_role_input};

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
        let mut server = tx.load_server().await?;
        let role = server.create_role(&actor, name.trim().to_string(), color, permissions, Utc::now())?;
        tx.insert_role(&role).await?;
        let events: Vec<MessagingEvent> = server.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(role)
    }
}
