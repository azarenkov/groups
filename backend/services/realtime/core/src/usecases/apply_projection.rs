use contracts::events::{
    friendship::{
        FriendRequestAcceptedV1 as FriendAcceptedDto, FriendRequestSentV1 as FriendRequestSentDto,
        FriendshipRemovedV1 as FriendRemovedDto,
    },
    server::{ServerCreatedV1 as ServerCreatedDto, ServerDeletedV1 as ServerIdDto,
        ServerMemberV1 as ServerMemberDto},
    user::UserLifecycleV1 as UserNameDto,
};
use ports::repository::RepositoryError;

use crate::ports::projection::RealtimeProjection;

pub struct ApplyProjection<R: RealtimeProjection> {
    projection: std::sync::Arc<R>,
}

impl<R: RealtimeProjection> ApplyProjection<R> {
    pub fn new(projection: std::sync::Arc<R>) -> Self {
        Self { projection }
    }

    pub async fn execute(&self, subject: &str, payload: &[u8]) -> Result<(), RepositoryError> {
    let resolver = self.projection.as_ref();
    if subject.ends_with(".user.registered") || subject.ends_with(".user.renamed") {
        if let Ok(evt) = serde_json::from_slice::<UserNameDto>(payload)
            && let Some(name) = &evt.name
        {
            resolver
                .upsert_user_name(evt.user_id, name, evt.occurred_at)
                .await?;
        }
        return Ok(());
    }
    if subject.ends_with(".user.request_sent") {
        if let Ok(evt) = serde_json::from_slice::<FriendRequestSentDto>(payload) {
            resolver
                .add_pending_friend_edge(evt.sender_id, evt.receiver_id, evt.occurred_at)
                .await?;
        }
        return Ok(());
    }
    if subject.ends_with(".user.request_accepted") {
        if let Ok(evt) = serde_json::from_slice::<FriendAcceptedDto>(payload) {
            resolver
                .add_friend_edge(evt.accepter_id, evt.sender_id, evt.occurred_at)
                .await?;
        }
        return Ok(());
    }
    if subject.ends_with(".user.removed") {
        if let Ok(evt) = serde_json::from_slice::<FriendRemovedDto>(payload) {
            resolver
                .remove_friend_edge(evt.user_id, evt.other_id, evt.occurred_at)
                .await?;
        }
        return Ok(());
    }
    if subject.ends_with(".server.created") {
        if let Ok(evt) = serde_json::from_slice::<ServerCreatedDto>(payload) {
            resolver
                .add_server_member(evt.server_id, evt.owner_id, evt.occurred_at)
                .await?;
        }
        return Ok(());
    }
    if subject.ends_with(".server.member_joined") {
        if let Ok(evt) = serde_json::from_slice::<ServerMemberDto>(payload) {
            resolver
                .add_server_member(evt.server_id, evt.user_id, evt.occurred_at)
                .await?;
        }
        return Ok(());
    }
    if subject.ends_with(".server.member_left") {
        if let Ok(evt) = serde_json::from_slice::<ServerMemberDto>(payload) {
            resolver
                .remove_server_member(evt.server_id, evt.user_id, evt.occurred_at)
                .await?;
        }
        return Ok(());
    }
    if subject.ends_with(".server.deleted") {
        if let Ok(evt) = serde_json::from_slice::<ServerIdDto>(payload) {
            resolver.remove_server(evt.server_id, evt.occurred_at).await?;
        }
        return Ok(());
    }
    Ok(())
}
}
