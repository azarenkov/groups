use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, community::id::CommunityId, community::unit_of_work::CommunityUnitOfWork, community::role::CommunityRole};

use super::{RoleError, validate_role_input};

pub struct CreateRole<U: CommunityUnitOfWork> {
    unit_of_work: Arc<U>,
}

impl<U: CommunityUnitOfWork> CreateRole<U> {
    pub fn new(unit_of_work: Arc<U>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        actor: UserId,
        name: String,
        color: String,
        permissions: i64,
    ) -> Result<CommunityRole, RoleError> {
        validate_role_input(&name, &color)?;
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let mut community = tx.load_community().await?;
        let role = community.create_role(&actor, name.trim().to_string(), color, permissions, Utc::now())?;
        tx.insert_role(&role).await?;
        let events: Vec<MessagingEvent> = community.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(role)
    }
}
