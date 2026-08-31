use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{privacy::ContactPolicy, privacy::PrivacyRepository, events::MessagingEvent, dm_message::DmMessage, dm_message::repository::DmMessageRepository, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, message::MessageError, message::content::MessageContent, server::id::ServerId, server::policy::PolicyError, server::repository::ServerRepository, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::dto::{RequestContext, responses::dm::send_dm_message::SendDmMessageResponse};

#[derive(Debug, thiserror::Error)]
pub enum SendServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message error: {0}")]
    Domain(#[from] MessageError),
    #[error("Not friends with this user")]
    NotFriends,
    #[error("This user does not accept direct messages")]
    DmsClosed,
    #[error("Server not found")]
    ServerNotFound,
    #[error("Missing permission to manage invites")]
    NotAllowed,
    #[error("Only server members can send invites")]
    NotServerMember,
    #[error("This user is already a member of the server")]
    AlreadyMember,
    #[error("You've already invited this user to this server")]
    InviteAlreadyPending,
}

impl From<PolicyError> for SendServerInviteError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => SendServerInviteError::NotServerMember,
            _ => SendServerInviteError::NotAllowed,
        }
    }
}

pub struct SendServerInvite<M, F, S, W, P>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
    S: ServerRepository,
    W: ServerUnitOfWork,
    P: PrivacyRepository,
{
    dm_repository: Arc<M>,
    friendship_repository: Arc<F>,
    server_repository: Arc<S>,
    unit_of_work: Arc<W>,
    privacy_repository: Arc<P>,
}

impl<M, F, S, W, P> SendServerInvite<M, F, S, W, P>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
    S: ServerRepository,
    W: ServerUnitOfWork,
    P: PrivacyRepository,
{
    pub fn new(
        dm_repository: Arc<M>,
        friendship_repository: Arc<F>,
        server_repository: Arc<S>,
        unit_of_work: Arc<W>,
        privacy_repository: Arc<P>,
    ) -> Self {
        Self {
            dm_repository,
            friendship_repository,
            server_repository,
            unit_of_work,
            privacy_repository,
        }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        friend_id: UserId,
        sender_id: UserId,
        ctx: RequestContext,
    ) -> Result<SendDmMessageResponse, SendServerInviteError> {
        let pair = self
            .friendship_repository
            .find_pair(&sender_id, &friend_id)
            .await?;
        if !matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted) {
            return Err(SendServerInviteError::NotFriends);
        }

        if matches!(
            self.privacy_repository.get(&friend_id).await?.dm_policy,
            ContactPolicy::NoOne
        ) {
            return Err(SendServerInviteError::DmsClosed);
        }

        let server = self
            .server_repository
            .find_server_by_id(&server_id)
            .await?
            .ok_or(SendServerInviteError::ServerNotFound)?;

        if self
            .dm_repository
            .has_pending_server_invite(&sender_id, &friend_id, &server_id)
            .await?
        {
            return Err(SendServerInviteError::InviteAlreadyPending);
        }

        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let membership = tx.load_server().await?;
        membership.ensure_can_manage_invites(&sender_id)?;
        if membership.is_member(&friend_id) {
            return Err(SendServerInviteError::AlreadyMember);
        }

        let server_name = server.name().as_str().to_string();
        let content = MessageContent::new(server_name.clone())?;
        let now = Utc::now();
        let mut message =
            DmMessage::create_server_invite(sender_id, friend_id, server_id, content, now);

        let request_id = ctx.request_id.clone();
        let events = message.pull_events();
        let events: Vec<MessagingEvent> = events.into_iter().map(MessagingEvent::from).collect();
        let persisted = async {
            tx.insert_dm_message(&message).await?;
            tx.save_events(&events, request_id.as_deref()).await?;
            tx.commit().await
        }
        .await;
        match persisted {
            Ok(()) => {}
            Err(RepositoryError::AlreadyExists) => {
                return Err(SendServerInviteError::InviteAlreadyPending);
            }
            Err(e) => return Err(SendServerInviteError::Repository(e)),
        }

        Ok(SendDmMessageResponse::from_invite(&message, server_name))
    }
}
