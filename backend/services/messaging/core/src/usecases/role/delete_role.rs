use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, community::id::CommunityId, community::unit_of_work::CommunityUnitOfWork};
use uuid::Uuid;

use super::{RoleError};

pub struct DeleteRole<U: CommunityUnitOfWork> {
    unit_of_work: Arc<U>,
}

impl<U: CommunityUnitOfWork> DeleteRole<U> {
    pub fn new(unit_of_work: Arc<U>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        actor: UserId,
        role_id: Uuid,
    ) -> Result<(), RoleError> {
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let mut community = tx.load_community().await?;
        community.delete_role(&actor, role_id, Utc::now())?;
        if !tx.delete_role(role_id).await? {
            return Err(RoleError::RoleNotFound);
        }
        let events: Vec<MessagingEvent> = community.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
