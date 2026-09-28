use std::sync::Arc;

use domain::{attachment::AttachmentLink, user::id::UserId};
use messaging_domain::voice_listen::repository::VoiceListenRepository;
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum VoiceListenError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a voice message")]
    NotVoice,
}

pub struct VoiceListens<R: VoiceListenRepository> {
    repo: Arc<R>,
}

impl<R: VoiceListenRepository> VoiceListens<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn mark(
        &self,
        message_id: Uuid,
        author_id: Uuid,
        listener: UserId,
        attachments: &[AttachmentLink],
        notify: &[Uuid],
    ) -> Result<(), VoiceListenError> {
        if !attachments.iter().any(|a| a.voice.is_some()) {
            return Err(VoiceListenError::NotVoice);
        }
        if author_id == listener.uuid() {
            return Ok(());
        }
        self.repo.mark_listened(message_id, &listener, notify).await?;
        Ok(())
    }

    pub async fn in_channel(
        &self,
        message_ids: &[Uuid],
        viewer: UserId,
    ) -> Result<Vec<Uuid>, VoiceListenError> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.repo.listened_in_channel(message_ids, &viewer).await?)
    }

    pub async fn in_dm(
        &self,
        message_ids: &[Uuid],
        viewer: UserId,
    ) -> Result<Vec<Uuid>, VoiceListenError> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.repo.listened_in_dm(message_ids, &viewer).await?)
    }
}
