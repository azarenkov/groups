use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::dm_conversation::{ClearScope, DmConversationRepository};
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DmConversationError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Cannot do this with a conversation with yourself")]
    SelfConversation,
    #[error("Conversation not found")]
    NotFound,
}

pub struct DmConversations<R: DmConversationRepository> {
    repo: Arc<R>,
}

impl<R: DmConversationRepository> DmConversations<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn clear(
        &self,
        actor: UserId,
        peer: Uuid,
        for_everyone: bool,
    ) -> Result<DateTime<Utc>, DmConversationError> {
        let peer = Self::peer(&actor, peer)?;
        let scope = if for_everyone { ClearScope::Everyone } else { ClearScope::Me };
        self.repo.clear(&actor, &peer, scope).await.map_err(Self::map)
    }

    pub async fn set_hidden(&self, user: UserId, peer: Uuid, hidden: bool) -> Result<(), DmConversationError> {
        let peer = Self::peer(&user, peer)?;
        self.repo.set_hidden(&user, &peer, hidden).await.map_err(Self::map)
    }

    pub async fn list_hidden(&self, user: UserId) -> Result<Vec<Uuid>, DmConversationError> {
        Ok(self.repo.list_hidden(&user).await?)
    }

    fn peer(user: &UserId, peer: Uuid) -> Result<UserId, DmConversationError> {
        if peer == user.uuid() {
            return Err(DmConversationError::SelfConversation);
        }
        Ok(UserId::new(peer))
    }

    fn map(e: RepositoryError) -> DmConversationError {
        match e {
            RepositoryError::NotFound => DmConversationError::NotFound,
            other => DmConversationError::Repository(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;

    #[derive(Default)]
    struct FakeRepo {
        calls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl DmConversationRepository for FakeRepo {
        async fn clear(&self, _actor: &UserId, _peer: &UserId, scope: ClearScope) -> Result<DateTime<Utc>, RepositoryError> {
            self.calls.lock().unwrap().push(format!("clear:{}", scope.as_str()));
            Ok(Utc::now())
        }

        async fn set_hidden(&self, _user: &UserId, _peer: &UserId, hidden: bool) -> Result<(), RepositoryError> {
            self.calls.lock().unwrap().push(format!("hidden:{hidden}"));
            Ok(())
        }

        async fn list_hidden(&self, _user: &UserId) -> Result<Vec<Uuid>, RepositoryError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn clear_picks_the_scope_from_the_flag() {
        let repo = Arc::new(FakeRepo::default());
        let usecase = DmConversations::new(repo.clone());
        let me = UserId::new(Uuid::new_v4());
        usecase.clear(me, Uuid::new_v4(), false).await.unwrap();
        usecase.clear(me, Uuid::new_v4(), true).await.unwrap();
        assert_eq!(*repo.calls.lock().unwrap(), vec!["clear:me", "clear:everyone"]);
    }

    #[tokio::test]
    async fn rejects_a_conversation_with_yourself() {
        let repo = Arc::new(FakeRepo::default());
        let usecase = DmConversations::new(repo.clone());
        let me = UserId::new(Uuid::new_v4());
        assert!(matches!(usecase.clear(me, me.uuid(), true).await, Err(DmConversationError::SelfConversation)));
        assert!(matches!(usecase.set_hidden(me, me.uuid(), true).await, Err(DmConversationError::SelfConversation)));
        assert!(repo.calls.lock().unwrap().is_empty());
    }
}
