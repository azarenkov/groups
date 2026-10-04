use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::{
    channel::{id::ChannelId, repository::ChannelRepository, visibility::ChannelVisibility},
    community::{id::CommunityId, repository::CommunityRepository},
    shared::{
        SharedContentRepository, SharedCursor, SharedItem, SharedKind, SharedScope, clamp_limit, normalize_query,
    },
};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum ListSharedContentError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Unknown shared content kind")]
    InvalidKind,
    #[error("Invalid pagination cursor")]
    InvalidCursor,
    #[error("Not a member of the community")]
    NotCommunityMember,
    #[error("Not a member of the channel")]
    NotChannelMember,
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Channel is not a text channel")]
    NotTextChannel,
    #[error("Cannot list shared content with yourself")]
    SelfConversation,
}

pub struct SharedQuery {
    pub kind: String,
    pub query: Option<String>,
    pub before: Option<String>,
    pub limit: Option<i64>,
}

pub struct SharedPage {
    pub items: Vec<SharedItem>,
    pub next_cursor: Option<String>,
}

pub struct ListSharedContent<R, C, S>
where
    R: SharedContentRepository,
    C: ChannelRepository,
    S: CommunityRepository,
{
    shared_repository: Arc<R>,
    channel_repository: Arc<C>,
    community_repository: Arc<S>,
}

impl<R, C, S> ListSharedContent<R, C, S>
where
    R: SharedContentRepository,
    C: ChannelRepository,
    S: CommunityRepository,
{
    pub fn new(shared_repository: Arc<R>, channel_repository: Arc<C>, community_repository: Arc<S>) -> Self {
        Self {
            shared_repository,
            channel_repository,
            community_repository,
        }
    }

    pub async fn in_channel(
        &self,
        requester_id: UserId,
        community_id: CommunityId,
        channel_id: ChannelId,
        query: SharedQuery,
    ) -> Result<SharedPage, ListSharedContentError> {
        self.ensure_readable(&community_id, &channel_id, &requester_id).await?;
        self.list(SharedScope::Channel(channel_id), requester_id, query).await
    }

    pub async fn in_dm(
        &self,
        requester_id: UserId,
        friend_id: UserId,
        query: SharedQuery,
    ) -> Result<SharedPage, ListSharedContentError> {
        if friend_id == requester_id {
            return Err(ListSharedContentError::SelfConversation);
        }
        let scope = SharedScope::Dm {
            user_a: requester_id,
            user_b: friend_id,
        };
        self.list(scope, requester_id, query).await
    }

    async fn list(
        &self,
        scope: SharedScope,
        viewer: UserId,
        query: SharedQuery,
    ) -> Result<SharedPage, ListSharedContentError> {
        let kind = SharedKind::parse(&query.kind).ok_or(ListSharedContentError::InvalidKind)?;
        let before = query
            .before
            .as_deref()
            .map(|raw| SharedCursor::decode(raw, kind).ok_or(ListSharedContentError::InvalidCursor))
            .transpose()?;
        let search = normalize_query(query.query.as_deref());
        let limit = clamp_limit(query.limit);
        let mut items = self
            .shared_repository
            .list_shared(&scope, &viewer, kind, search.as_deref(), before.as_ref(), limit + 1)
            .await?;
        let has_more = items.len() as i64 > limit;
        items.truncate(limit as usize);
        let next_cursor = if has_more {
            items.last().map(|item| item.cursor.encode())
        } else {
            None
        };
        Ok(SharedPage { items, next_cursor })
    }

    async fn ensure_readable(
        &self,
        community_id: &CommunityId,
        channel_id: &ChannelId,
        requester_id: &UserId,
    ) -> Result<(), ListSharedContentError> {
        if !self
            .community_repository
            .is_community_member(community_id, requester_id)
            .await?
        {
            return Err(ListSharedContentError::NotCommunityMember);
        }
        let channel = self
            .channel_repository
            .find_channel_by_id(channel_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => ListSharedContentError::ChannelNotFound,
                other => ListSharedContentError::Repository(other),
            })?;
        if channel.community_id() != community_id {
            return Err(ListSharedContentError::ChannelNotFound);
        }
        if !channel.supports_text() {
            return Err(ListSharedContentError::NotTextChannel);
        }
        if channel.visibility() == ChannelVisibility::Private
            && !self
                .channel_repository
                .is_text_channel_member(channel_id, requester_id)
                .await?
        {
            return Err(ListSharedContentError::NotChannelMember);
        }
        Ok(())
    }
}
