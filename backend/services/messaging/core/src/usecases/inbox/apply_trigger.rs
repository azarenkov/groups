use std::sync::Arc;

use chrono::{DateTime, Utc};
use messaging_domain::inbox::{InboxEffect, InboxRepository, InboxRetention, InboxTrigger};
use ports::repository::RepositoryError;

pub struct ApplyInboxTrigger<R: InboxRepository> {
    repository: Arc<R>,
    retention: InboxRetention,
}

impl<R: InboxRepository> ApplyInboxTrigger<R> {
    pub fn new(repository: Arc<R>, retention: InboxRetention) -> Self {
        Self { repository, retention }
    }

    pub async fn execute(
        &self,
        trigger: &InboxTrigger,
        occurred_at: DateTime<Utc>,
        source_event_id: &str,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        match trigger.effect(occurred_at) {
            Some(InboxEffect::Deliver(notification)) if !self.retention.admits(&notification, now) => {}
            Some(InboxEffect::Deliver(notification)) => {
                self.repository.deliver(&notification, source_event_id).await?;
            }
            Some(InboxEffect::WithdrawInvitation { recipient_id, invitation_id }) => {
                self.repository.withdraw_invitation(&recipient_id, &invitation_id).await?;
            }
            None => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use domain::user::id::UserId;
    use messaging_domain::community::{id::CommunityId, invitation::InvitationId};
    use messaging_domain::inbox::{InboxCursor, InboxItem, Notification};
    use uuid::Uuid;

    use super::*;

    #[derive(Default)]
    struct Calls(Mutex<Vec<String>>);

    #[async_trait]
    impl InboxRepository for Calls {
        async fn deliver(&self, n: &Notification, source: &str) -> Result<bool, RepositoryError> {
            self.0.lock().unwrap().push(format!("deliver:{}:{source}:{}", n.kind.as_str(), n.created_at.timestamp()));
            Ok(true)
        }
        async fn withdraw_invitation(&self, _: &UserId, _: &InvitationId) -> Result<bool, RepositoryError> {
            self.0.lock().unwrap().push("withdraw".to_string());
            Ok(true)
        }
        async fn list(&self, _: &UserId, _: Option<&InboxCursor>, _: i64) -> Result<Vec<InboxItem>, RepositoryError> {
            unimplemented!()
        }
        async fn unread_count(&self, _: &UserId) -> Result<i64, RepositoryError> {
            unimplemented!()
        }
        async fn mark_read(&self, _: &UserId, _: Option<&[Uuid]>, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn expire(&self, _: DateTime<Utc>, _: &[&str], _: i64) -> Result<u64, RepositoryError> {
            unimplemented!()
        }
        async fn trim(&self, _: i64, _: &[&str], _: i64) -> Result<u64, RepositoryError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn turns_triggers_into_deliveries_withdrawals_or_nothing() {
        let calls = Arc::new(Calls::default());
        let usecase = ApplyInboxTrigger::new(calls.clone(), InboxRetention::default());
        let (a, b) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let community_id = CommunityId::new(Uuid::new_v4());
        let at = DateTime::from_timestamp(1_790_000_000, 0).unwrap();
        usecase.execute(&InboxTrigger::FriendRequestSent { sender_id: a, receiver_id: b }, at, "e1", at).await.unwrap();
        usecase
            .execute(&InboxTrigger::InvitationResolved { invitation_id: InvitationId::generate(), invitee_id: b }, at, "e2", at)
            .await
            .unwrap();
        usecase.execute(&InboxTrigger::MemberRemoved { community_id, user_id: a, removed_by: None }, at, "e3", at).await.unwrap();
        usecase
            .execute(&InboxTrigger::FriendRequestSent { sender_id: a, receiver_id: b }, at, "e4", at + chrono::Duration::days(31))
            .await
            .unwrap();
        assert_eq!(*calls.0.lock().unwrap(), vec!["deliver:friend_request_received:e1:1790000000".to_string(), "withdraw".to_string()]);
    }
}
