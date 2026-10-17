use contracts::events::{
    dm_message::{
        DmConversationClearedV1 as DmClearedDto, DmMessageDeletedV1 as DmDeletedDto, DmMessageSentV1 as DmSentDto,
    },
    friendship::{
        FriendRequestAcceptedV1 as FriendAcceptedDto, FriendRequestSentV1 as FriendRequestSentDto,
        FriendshipRemovedV1 as FriendRemovedDto,
    },
    community::{CommunityCreatedV1 as CommunityCreatedDto, CommunityDeletedV1 as CommunityIdDto,
        CommunityMemberV1 as CommunityMemberDto},
    user::UserLifecycleV1 as UserNameDto,
};
use ports::repository::RepositoryError;

use crate::ports::{blocks::RelationDirectory, projection::RealtimeProjection};

pub struct ApplyProjection<R: RealtimeProjection, B: RelationDirectory> {
    projection: std::sync::Arc<R>,
    relations: std::sync::Arc<B>,
}

impl<R: RealtimeProjection, B: RelationDirectory> ApplyProjection<R, B> {
    pub fn new(projection: std::sync::Arc<R>, relations: std::sync::Arc<B>) -> Self {
        Self { projection, relations }
    }

    async fn forget_pair_without_history(&self, a: uuid::Uuid, b: uuid::Uuid) -> Result<(), RepositoryError> {
        let remaining = self
            .relations
            .dm_peers(a, &[b])
            .await
            .map_err(|e| RepositoryError::ConnectionError(e.to_string()))?;
        if remaining.is_empty() {
            self.projection.remove_dm_pair(a, b).await?;
        }
        Ok(())
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
    if subject.ends_with(".dm_message.sent") {
        if let Ok(evt) = serde_json::from_slice::<DmSentDto>(payload) {
            resolver.record_dm_pair(evt.sender_id, evt.recipient_id).await?;
        }
        return Ok(());
    }
    if subject.ends_with(".dm_message.deleted") {
        if let Ok(evt) = serde_json::from_slice::<DmDeletedDto>(payload) {
            self.forget_pair_without_history(evt.sender_id, evt.recipient_id).await?;
        }
        return Ok(());
    }
    if subject.ends_with(".dm_conversation.cleared") {
        if let Ok(evt) = serde_json::from_slice::<DmClearedDto>(payload) {
            self.forget_pair_without_history(evt.sender_id, evt.recipient_id).await?;
        }
        return Ok(());
    }
    if subject.ends_with(".community.created") {
        if let Ok(evt) = serde_json::from_slice::<CommunityCreatedDto>(payload) {
            resolver
                .add_community_member(evt.community_id, evt.owner_id, evt.occurred_at)
                .await?;
        }
        return Ok(());
    }
    if subject.ends_with(".community.member_joined") {
        if let Ok(evt) = serde_json::from_slice::<CommunityMemberDto>(payload) {
            resolver
                .add_community_member(evt.community_id, evt.user_id, evt.occurred_at)
                .await?;
        }
        return Ok(());
    }
    if subject.ends_with(".community.member_left") {
        if let Ok(evt) = serde_json::from_slice::<CommunityMemberDto>(payload) {
            resolver
                .remove_community_member(evt.community_id, evt.user_id, evt.occurred_at)
                .await?;
        }
        return Ok(());
    }
    if subject.ends_with(".community.deleted") {
        if let Ok(evt) = serde_json::from_slice::<CommunityIdDto>(payload) {
            resolver.remove_community(evt.community_id, evt.occurred_at).await?;
        }
        return Ok(());
    }
    Ok(())
}
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use super::*;
    use crate::ports::blocks::{BlockSets, RelationLookupError};

    #[derive(Default)]
    struct FakeProjection {
        removed: Mutex<Vec<(Uuid, Uuid)>>,
    }

    #[async_trait]
    impl RealtimeProjection for FakeProjection {
        async fn upsert_user_name(&self, _: Uuid, _: &str, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
        async fn add_friend_edge(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
        async fn add_pending_friend_edge(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
        async fn remove_friend_edge(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
        async fn record_dm_pair(&self, _: Uuid, _: Uuid) -> Result<(), RepositoryError> { Ok(()) }
        async fn remove_dm_pair(&self, a: Uuid, b: Uuid) -> Result<(), RepositoryError> {
            self.removed.lock().unwrap().push((a, b));
            Ok(())
        }
        async fn add_community_member(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
        async fn remove_community_member(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
        async fn remove_community(&self, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
        async fn set_appear_offline(&self, _: Uuid, _: bool) -> Result<(), RepositoryError> { Ok(()) }
        async fn appear_offline(&self, _: Uuid) -> Result<bool, RepositoryError> { Ok(false) }
    }

    struct FakeRelations {
        peers: Vec<Uuid>,
    }

    #[async_trait]
    impl RelationDirectory for FakeRelations {
        async fn block_relations(&self, _: Uuid, _: &[Uuid]) -> Result<BlockSets, RelationLookupError> {
            Ok(BlockSets::default())
        }
        async fn dm_peers(&self, _: Uuid, candidates: &[Uuid]) -> Result<Vec<Uuid>, RelationLookupError> {
            Ok(candidates.iter().copied().filter(|c| self.peers.contains(c)).collect())
        }
    }

    fn cleared(a: Uuid, b: Uuid) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "sender_id": a,
            "recipient_id": b,
            "scope": "everyone",
            "cleared_at": Utc::now(),
            "recipients": [a, b],
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn clearing_all_history_forgets_the_dm_pair() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let projection = Arc::new(FakeProjection::default());
        let apply = ApplyProjection::new(projection.clone(), Arc::new(FakeRelations { peers: vec![] }));
        apply.execute("messaging.dm_message.p01.dm_conversation.cleared", &cleared(a, b)).await.unwrap();
        assert_eq!(*projection.removed.lock().unwrap(), vec![(a, b)]);
    }

    #[tokio::test]
    async fn a_pair_with_remaining_history_is_kept() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let projection = Arc::new(FakeProjection::default());
        let apply = ApplyProjection::new(projection.clone(), Arc::new(FakeRelations { peers: vec![b] }));
        apply.execute("messaging.dm_message.p01.dm_conversation.cleared", &cleared(a, b)).await.unwrap();
        assert!(projection.removed.lock().unwrap().is_empty());
    }
}
