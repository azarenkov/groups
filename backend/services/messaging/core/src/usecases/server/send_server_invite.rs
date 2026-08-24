use std::sync::Arc;

use chrono::Utc;
use domain::{
    dm_message::{DmMessage, repository::DmMessageRepository},
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    message::{MessageError, content::MessageContent},
    server::{id::ServerId, repository::ServerRepository},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

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

pub struct SendServerInvite<M, F, S, R>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
    S: ServerRepository,
    R: domain::server::role::ServerRoleRepository,
{
    dm_repository: Arc<M>,
    friendship_repository: Arc<F>,
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<M, F, S, R> SendServerInvite<M, F, S, R>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
    S: ServerRepository,
    R: domain::server::role::ServerRoleRepository,
{
    pub fn new(
        dm_repository: Arc<M>,
        friendship_repository: Arc<F>,
        server_repository: Arc<S>,
        role_repository: Arc<R>,
    ) -> Self {
        Self {
            dm_repository,
            friendship_repository,
            server_repository,
            role_repository,
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

        if !self
            .server_repository
            .is_server_member(&server_id, &sender_id)
            .await?
        {
            return Err(SendServerInviteError::NotServerMember);
        }

        let owner = self.server_repository.find_server_owner(&server_id).await?;
        let allowed = owner.as_ref() == Some(&sender_id)
            || domain::server::role::has_permission(
                self.role_repository
                    .user_permissions(&server_id, &sender_id)
                    .await?,
                domain::server::role::PERM_MANAGE_INVITES,
            );
        if !allowed {
            return Err(SendServerInviteError::NotAllowed);
        }

        if self
            .server_repository
            .is_server_member(&server_id, &friend_id)
            .await?
        {
            return Err(SendServerInviteError::AlreadyMember);
        }

        if self
            .dm_repository
            .has_pending_server_invite(&sender_id, &friend_id, &server_id)
            .await?
        {
            return Err(SendServerInviteError::InviteAlreadyPending);
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
        match self
            .dm_repository
            .save_with_events(&message, &records, &[])
            .await
        {
            Ok(()) => {}
            Err(RepositoryError::AlreadyExists) => {
                return Err(SendServerInviteError::InviteAlreadyPending);
            }
            Err(e) => return Err(SendServerInviteError::Repository(e)),
        }

        Ok(SendDmMessageResponse::from_invite(&message, server_name))
    }
}
