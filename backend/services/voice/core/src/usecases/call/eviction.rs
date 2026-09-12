use chrono::Utc;
use domain::{channel::id::ChannelId, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;
use voice_domain::call::repository::CallRepository;

use crate::ports::voice_service::VoiceService;

pub async fn evict<C: CallRepository, V: VoiceService>(
    call_repository: &C,
    voice_service: &V,
    call_id: Uuid,
    users: &[UserId],
    request_id: Option<String>,
) -> Result<(), RepositoryError> {
    if users.is_empty() {
        return Ok(());
    }
    call_repository
        .evict_many(call_id, users, Utc::now(), request_id)
        .await?;
    let room = ChannelId::new(call_id);
    for user in users {
        match voice_service.remove_participant(&room, user).await {
            Ok(()) => {
                if let Err(e) = call_repository.eviction_done(call_id, user).await {
                    tracing::warn!(error = %e, %call_id, user_id = %user.uuid(), "Failed to clear a completed eviction");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, %call_id, user_id = %user.uuid(), "Room removal failed; eviction queued for retry");
            }
        }
    }
    Ok(())
}
