use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, community::id::CommunityId, community::unit_of_work::CommunityUnitOfWork};
use uuid::Uuid;

use super::{RoleError};

pub struct SwapRoles<U: CommunityUnitOfWork> {
    unit_of_work: Arc<U>,
}

impl<U: CommunityUnitOfWork> SwapRoles<U> {
    pub fn new(unit_of_work: Arc<U>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        actor: UserId,
        role_a: Uuid,
        role_b: Uuid,
    ) -> Result<(), RoleError> {
        if role_a == role_b {
            return Err(RoleError::InvalidData);
        }
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let mut community = tx.load_community().await?;
        community.swap_roles(&actor, role_a, role_b, Utc::now())?;
        if !tx.swap_role_positions(role_a, role_b).await? {
            return Err(RoleError::RoleNotFound);
        }
        let events: Vec<MessagingEvent> = community.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
