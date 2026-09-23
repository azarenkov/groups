use messaging_domain::message::system::SystemMessageKind;

use crate::usecases::community::membership_announcements::announce_membership;
use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;

use messaging_domain::{events::MessagingEvent, community::id::CommunityId, community::policy::PolicyError, community::unit_of_work::CommunityUnitOfWork};

#[derive(Debug, thiserror::Error)]
pub enum LeaveCommunityError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("You are not a member of this community")]
    NotMember,
    #[error("Community owners cannot leave their own community; delete it instead")]
    OwnerCannotLeave,
}

impl From<PolicyError> for LeaveCommunityError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => LeaveCommunityError::NotMember,
            _ => LeaveCommunityError::OwnerCannotLeave,
        }
    }
}

pub struct LeaveCommunity<W: CommunityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: CommunityUnitOfWork> LeaveCommunity<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        user_id: UserId,
    ) -> Result<(), LeaveCommunityError> {
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let mut community = tx.load_community().await?;
        community.leave(&user_id, chrono::Utc::now())?;
        if !tx.remove_member(&user_id).await? {
            return Err(LeaveCommunityError::NotMember);
        }
        let now = chrono::Utc::now();
        let mut events: Vec<MessagingEvent> = community.pull_events().into_iter().map(MessagingEvent::from).collect();
        events.extend(announce_membership(&mut *tx, user_id, SystemMessageKind::MemberLeft, now).await?);
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
