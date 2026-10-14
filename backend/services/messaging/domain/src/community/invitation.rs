use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{event::EventId, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::community::{Community, events::CommunityEvent, id::CommunityId, policy::PolicyError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvitationId(Uuid);

impl InvitationId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InvitationError {
    #[error("Only community members can send invites")]
    NotMember,
    #[error("Missing permission to manage invites")]
    NotAllowed,
    #[error("This user is already a member of the community")]
    AlreadyMember,
    #[error("A user cannot invite themselves")]
    SelfInvite,
    #[error("This invite was not addressed to you")]
    NotInvitee,
}

impl From<PolicyError> for InvitationError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => Self::NotMember,
            _ => Self::NotAllowed,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Invitation {
    id: InvitationId,
    community_id: CommunityId,
    inviter_id: UserId,
    invitee_id: UserId,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvitationResolution {
    Accepted,
    Declined,
}

impl Invitation {
    pub fn issue(
        community: &Community,
        inviter_id: UserId,
        invitee_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<(Self, CommunityEvent), InvitationError> {
        if inviter_id == invitee_id {
            return Err(InvitationError::SelfInvite);
        }
        community.ensure_can_manage_invites(&inviter_id)?;
        if community.is_member(&invitee_id) {
            return Err(InvitationError::AlreadyMember);
        }
        let invitation = Self {
            id: InvitationId::generate(),
            community_id: *community.id(),
            inviter_id,
            invitee_id,
            created_at: now,
        };
        let event = CommunityEvent::InvitationIssued(CommunityInvitationIssued {
            id: EventId::generate(),
            invitation_id: invitation.id,
            community_id: invitation.community_id,
            inviter_id,
            invitee_id,
            occurred_at: now,
        });
        Ok((invitation, event))
    }

    pub fn restore(
        id: InvitationId,
        community_id: CommunityId,
        inviter_id: UserId,
        invitee_id: UserId,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self { id, community_id, inviter_id, invitee_id, created_at }
    }

    pub fn resolve(
        &self,
        actor: &UserId,
        resolution: InvitationResolution,
        now: DateTime<Utc>,
    ) -> Result<CommunityEvent, InvitationError> {
        if actor != &self.invitee_id {
            return Err(InvitationError::NotInvitee);
        }
        Ok(CommunityEvent::InvitationResolved(CommunityInvitationResolved {
            id: EventId::generate(),
            invitation_id: self.id,
            community_id: self.community_id,
            invitee_id: self.invitee_id,
            accepted: resolution == InvitationResolution::Accepted,
            occurred_at: now,
        }))
    }

    pub fn settle_by_joining(&self, now: DateTime<Utc>) -> CommunityEvent {
        CommunityEvent::InvitationResolved(CommunityInvitationResolved {
            id: EventId::generate(),
            invitation_id: self.id,
            community_id: self.community_id,
            invitee_id: self.invitee_id,
            accepted: true,
            occurred_at: now,
        })
    }

    pub fn id(&self) -> &InvitationId {
        &self.id
    }

    pub fn community_id(&self) -> &CommunityId {
        &self.community_id
    }

    pub fn inviter_id(&self) -> &UserId {
        &self.inviter_id
    }

    pub fn invitee_id(&self) -> &UserId {
        &self.invitee_id
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Debug)]
pub struct CommunityInvitationIssued {
    pub id: EventId,
    pub invitation_id: InvitationId,
    pub community_id: CommunityId,
    pub inviter_id: UserId,
    pub invitee_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CommunityInvitationResolved {
    pub id: EventId,
    pub invitation_id: InvitationId,
    pub community_id: CommunityId,
    pub invitee_id: UserId,
    pub accepted: bool,
    pub occurred_at: DateTime<Utc>,
}

#[async_trait]
pub trait InvitationRepository: Send + Sync {
    async fn find(&self, id: &InvitationId) -> Result<Option<Invitation>, RepositoryError>;
    async fn is_pending(&self, community_id: &CommunityId, invitee_id: &UserId) -> Result<bool, RepositoryError>;
    async fn pending_invitees(&self, community_id: &CommunityId) -> Result<Vec<UserId>, RepositoryError>;
}

#[async_trait]
pub trait LegacyInvitations: Send + Sync {
    async fn adopt(&self, limit: i64) -> Result<u64, RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joining_another_way_settles_the_invitation_as_accepted() {
        let invitee = UserId::new(Uuid::new_v4());
        let invitation = Invitation::restore(
            InvitationId::generate(),
            CommunityId::new(Uuid::new_v4()),
            UserId::new(Uuid::new_v4()),
            invitee,
            Utc::now(),
        );
        let now = Utc::now();
        match invitation.settle_by_joining(now) {
            CommunityEvent::InvitationResolved(e) => {
                assert_eq!(e.invitation_id, *invitation.id());
                assert_eq!(e.invitee_id, invitee);
                assert!(e.accepted);
                assert_eq!(e.occurred_at, now);
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
