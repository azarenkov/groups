use messaging_domain::message::system::SystemMessageKind;

use crate::usecases::community::membership_announcements::announce_membership;
use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;

use messaging_domain::{events::MessagingEvent, community::id::CommunityId, community::policy::PolicyError, community::unit_of_work::CommunityUnitOfWork};

#[derive(Debug, thiserror::Error)]
pub enum KickMemberError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Target is not a member of this community")]
    NotMember,
    #[error("Missing permission to kick members")]
    NotAllowed,
}

impl From<PolicyError> for KickMemberError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::TargetNotMember => KickMemberError::NotMember,
            _ => KickMemberError::NotAllowed,
        }
    }
}

pub struct KickMember<W: CommunityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: CommunityUnitOfWork> KickMember<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        actor: UserId,
        target: UserId,
    ) -> Result<(), KickMemberError> {
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let mut community = tx.load_community().await?;
        community.kick_member(&actor, &target, chrono::Utc::now())?;
        if !tx.remove_member(&target).await? {
            return Err(KickMemberError::NotMember);
        }
        let now = chrono::Utc::now();
        let mut events: Vec<MessagingEvent> = community.pull_events().into_iter().map(MessagingEvent::from).collect();
        events.extend(announce_membership(&mut *tx, target, SystemMessageKind::MemberLeft, now).await?);
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
