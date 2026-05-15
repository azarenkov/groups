use std::sync::Arc;

use chrono::Utc;
use domain::{
    channel::{ChannelError, id::ChannelId, repository::ChannelRepository},
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

use crate::dto::requests::channel::join_text_channel::JoinTextChannelRequest;

#[derive(Debug, thiserror::Error)]
pub enum JoinTextChannelError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Not a member of the server")]
    NotServerMember,
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Channel is not a text channel")]
    NotTextChannel,
}

pub struct JoinTextChannel<C, S>
where
    C: ChannelRepository,
    S: ServerRepository,
{
    channel_repository: Arc<C>,
    server_repository: Arc<S>,
}

impl<C, S> JoinTextChannel<C, S>
where
    C: ChannelRepository,
    S: ServerRepository,
{
    pub fn new(channel_repository: Arc<C>, server_repository: Arc<S>) -> Self {
        Self {
            channel_repository,
            server_repository,
        }
    }

    pub async fn execute(
        &self,
        request: JoinTextChannelRequest,
        requester_id: UserId,
    ) -> Result<(), JoinTextChannelError> {
        let server_id = ServerId::new(request.server_id);
        let channel_id = ChannelId::new(request.channel_id);

        let is_member = self
            .server_repository
            .is_server_member(&server_id, &requester_id)
            .await?;
        if !is_member {
            return Err(JoinTextChannelError::NotServerMember);
        }

        let mut channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => JoinTextChannelError::ChannelNotFound,
                other => JoinTextChannelError::Repository(other),
            })?;

        if channel.server_id() != &server_id {
            return Err(JoinTextChannelError::ChannelNotFound);
        }

        if !channel.supports_text() {
            return Err(JoinTextChannelError::NotTextChannel);
        }

        let now = Utc::now();
        let added = channel.join_text(requester_id, now)?;
        if !added {
            return Ok(());
        }

        let request_id = request.ctx.request_id.clone();
        let records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        self.channel_repository
            .join_text_with_events(&channel, &requester_id, &records)
            .await?;

        Ok(())
    }
}
