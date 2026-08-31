use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{privacy::DmPolicy, privacy::PrivacyRepository, events::MessagingEvent, dm_message::DmMessage, dm_message::unit_of_work::DmUnitOfWork, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, message::MessageError, message::content::MessageContent};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum LogCallError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message error: {0}")]
    Domain(#[from] MessageError),
    #[error("Not friends with this user")]
    NotFriends,
    #[error("This user does not accept direct messages")]
    DmsClosed,
}

pub struct LogCall<W, F, P>
where
    W: DmUnitOfWork,
    F: FriendshipRepository,
    P: PrivacyRepository,
{
    unit_of_work: Arc<W>,
    friendship_repository: Arc<F>,
    privacy_repository: Arc<P>,
}

impl<W, F, P> LogCall<W, F, P>
where
    W: DmUnitOfWork,
    F: FriendshipRepository,
    P: PrivacyRepository,
{
    pub fn new(
        unit_of_work: Arc<W>,
        friendship_repository: Arc<F>,
        privacy_repository: Arc<P>,
    ) -> Self {
        Self {
            unit_of_work,
            friendship_repository,
            privacy_repository,
        }
    }

    pub async fn execute(
        &self,
        sender_id: UserId,
        recipient_id: Uuid,
        status: String,
        duration_seconds: Option<i32>,
    ) -> Result<(), LogCallError> {
        let recipient_id = UserId::new(recipient_id);

        let pair = self
            .friendship_repository
            .find_pair(&sender_id, &recipient_id)
            .await?;
        if !matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted) {
            return Err(LogCallError::NotFriends);
        }

        if matches!(
            self.privacy_repository.get(&recipient_id).await?.dm_policy,
            DmPolicy::NoOne
        ) {
            return Err(LogCallError::DmsClosed);
        }

        let content = MessageContent::new("call".to_string())?;
        let now = Utc::now();
        let mut message =
            DmMessage::create_call(sender_id, recipient_id, status, duration_seconds, content, now);
        let events = message.pull_events();
        let events: Vec<MessagingEvent> = events.into_iter().map(MessagingEvent::from).collect();
        let mut tx = self.unit_of_work.begin().await?;
        tx.insert_message(&message, &[]).await?;
        tx.save_events(&events, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
