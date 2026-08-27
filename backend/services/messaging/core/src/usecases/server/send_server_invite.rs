use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{dm_message::DmMessage, dm_message::repository::DmMessageRepository, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, message::MessageError, message::content::MessageContent, server::id::ServerId, server::policy::ModerationPolicy, server::policy::PolicyError, server::repository::ServerRepository, server::unit_of_work::ServerUnitOfWork};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord, shared::repository::error::RepositoryError, user::id::UserId};

use crate::dto::{RequestContext, responses::dm::send_dm_message::SendDmMessageResponse};

#[derive(Debug, thiserror::Error)]
pub enum SendServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message error: {0}")]
    Domain(#[from] MessageError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Not friends with this user")]
    NotFriends,
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

pub struct SendServerInvite<M, F, S, W>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
    S: ServerRepository,
    W: ServerUnitOfWork,
{
    dm_repository: Arc<M>,
    friendship_repository: Arc<F>,
    server_repository: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<M, F, S, W> SendServerInvite<M, F, S, W>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
    S: ServerRepository,
    W: ServerUnitOfWork,
{
    pub fn new(
        dm_repository: Arc<M>,
        friendship_repository: Arc<F>,
        server_repository: Arc<S>,
        unit_of_work: Arc<W>,
    ) -> Self {
        Self {
            dm_repository,
            friendship_repository,
            server_repository,
            unit_of_work,
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
        let snapshot = tx.authorization().await?;
        ModerationPolicy::manage_invites(&snapshot, &sender_id)?;
        if snapshot.is_member(&friend_id) {
            return Err(SendServerInviteError::AlreadyMember);
        }

        let server_name = server.name().as_str().to_string();
        let content = MessageContent::new(server_name.clone())?;
        let now = Utc::now();
        let mut message =
            DmMessage::create_server_invite(sender_id, friend_id, server_id, content, now);

        let request_id = ctx.request_id.clone();
        let mut records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        crate::usecases::enrichment::inject_field(
            &mut records,
            "invite_server_id",
            serde_json::json!(server_id.uuid()),
        );
        crate::usecases::enrichment::inject_field(
            &mut records,
            "invite_server_name",
            serde_json::json!(server_name.clone()),
        );
        let persisted = async {
            tx.insert_dm_message(&message).await?;
            tx.save_outbox(&records).await?;
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
