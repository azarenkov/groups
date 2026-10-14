use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::inbox::{InboxCursor, InboxItem, InboxRepository, clamp_inbox_limit};
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ReadInboxError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid pagination cursor")]
    InvalidCursor,
}

pub struct InboxPage {
    pub items: Vec<InboxItem>,
    pub next_cursor: Option<String>,
}

pub struct ReadInbox<R: InboxRepository> {
    repository: Arc<R>,
}

impl<R: InboxRepository> ReadInbox<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn list(&self, user: UserId, before: Option<&str>, limit: Option<i64>) -> Result<InboxPage, ReadInboxError> {
        let cursor = before.map(|raw| InboxCursor::decode(raw).ok_or(ReadInboxError::InvalidCursor)).transpose()?;
        let limit = clamp_inbox_limit(limit);
        let mut items = self.repository.list(&user, cursor.as_ref(), limit + 1).await?;
        let has_more = items.len() as i64 > limit;
        items.truncate(limit as usize);
        let next_cursor = if has_more { items.last().map(|item| InboxCursor::of(item).encode()) } else { None };
        Ok(InboxPage { items, next_cursor })
    }

    pub async fn unread_count(&self, user: UserId) -> Result<i64, ReadInboxError> {
        Ok(self.repository.unread_count(&user).await?)
    }

    pub async fn mark_read(&self, user: UserId, ids: Option<Vec<Uuid>>) -> Result<(), ReadInboxError> {
        Ok(self.repository.mark_read(&user, ids.as_deref(), Utc::now()).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, TimeZone};
    use messaging_domain::community::invitation::InvitationId;
    use messaging_domain::inbox::{InboxItemKind, Notification};

    use super::*;

    struct Items {
        items: Vec<InboxItem>,
        asked: Mutex<Vec<(Option<InboxCursor>, i64)>>,
    }

    #[async_trait]
    impl InboxRepository for Items {
        async fn deliver(&self, _: &Notification, _: &str) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn withdraw_invitation(&self, _: &UserId, _: &InvitationId) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn list(&self, _: &UserId, before: Option<&InboxCursor>, limit: i64) -> Result<Vec<InboxItem>, RepositoryError> {
            self.asked.lock().unwrap().push((before.copied(), limit));
            Ok(self.items.iter().take(limit as usize).cloned().collect())
        }
        async fn unread_count(&self, _: &UserId) -> Result<i64, RepositoryError> {
            Ok(3)
        }
        async fn mark_read(&self, _: &UserId, _: Option<&[Uuid]>, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    fn item(minute: u32) -> InboxItem {
        InboxItem {
            id: Uuid::new_v4(),
            kind: InboxItemKind::FriendRequestAccepted,
            actor_id: UserId::new(Uuid::new_v4()),
            actor_name: "bob".to_string(),
            created_at: Utc.with_ymd_and_hms(2026, 10, 14, 12, minute, 0).unwrap(),
            read: false,
        }
    }

    #[tokio::test]
    async fn pages_with_a_cursor_from_the_last_item() {
        let repository = Arc::new(Items { items: (0..5).rev().map(item).collect(), asked: Mutex::default() });
        let usecase = ReadInbox::new(repository.clone());
        let page = usecase.list(UserId::new(Uuid::new_v4()), None, Some(2)).await.unwrap();
        assert_eq!(page.items.len(), 2);
        let cursor = page.next_cursor.expect("more pages");
        assert_eq!(InboxCursor::decode(&cursor), Some(InboxCursor::of(&page.items[1])));
        assert_eq!(repository.asked.lock().unwrap()[0], (None, 3));
    }

    #[tokio::test]
    async fn rejects_a_malformed_cursor() {
        let repository = Arc::new(Items { items: vec![], asked: Mutex::default() });
        let usecase = ReadInbox::new(repository);
        assert!(matches!(
            usecase.list(UserId::new(Uuid::new_v4()), Some("garbage"), None).await,
            Err(ReadInboxError::InvalidCursor)
        ));
    }
}
