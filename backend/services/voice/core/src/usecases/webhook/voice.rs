use std::sync::Arc;

use domain::{channel::id::ChannelId, user::id::UserId};
use ports::{repository::RepositoryError, user_profile::UserProfileRepository};
use voice_domain::{call::repository::CallRepository, call::repository::WebhookJoinOutcome, webhook::WebhookEvent, webhook::WebhookKind};

use crate::{
    dto::context::RequestContext,
    ports::{
        friendship_lookup::{FriendshipLookup, FriendshipLookupError},
        voice_service::{VoiceService, VoiceServiceError},
        voice_webhook_verifier::{VoiceWebhookEvent, VoiceWebhookVerifier, VoiceWebhookVerifierError},
        webhook_outbox::WebhookOutbox,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum HandleVoiceWebhookError {
    #[error("Webhook verifier error: {0}")]
    Verifier(#[from] VoiceWebhookVerifierError),
    #[error("Call repository error: {0}")]
    Call(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
    #[error("Friendship lookup unavailable: {0}")]
    FriendshipLookup(#[from] FriendshipLookupError),
}

pub struct HandleVoiceWebhook<V, C, O, P, S, F>
where
    V: VoiceWebhookVerifier,
    C: CallRepository,
    O: WebhookOutbox,
    P: UserProfileRepository,
    S: VoiceService,
    F: FriendshipLookup,
{
    verifier: Arc<V>,
    call_repository: Arc<C>,
    outbox: Arc<O>,
    profiles: Arc<P>,
    voice_service: Arc<S>,
    friendship_repository: Arc<F>,
}

impl<V, C, O, P, S, F> HandleVoiceWebhook<V, C, O, P, S, F>
where
    V: VoiceWebhookVerifier,
    C: CallRepository,
    O: WebhookOutbox,
    P: UserProfileRepository,
    S: VoiceService,
    F: FriendshipLookup,
{
    pub fn new(
        verifier: Arc<V>,
        call_repository: Arc<C>,
        outbox: Arc<O>,
        profiles: Arc<P>,
        voice_service: Arc<S>,
        friendship_repository: Arc<F>,
    ) -> Self {
        Self {
            verifier,
            call_repository,
            outbox,
            profiles,
            voice_service,
            friendship_repository,
        }
    }

    async fn evict_if_blocked(
        &self,
        call_id: uuid::Uuid,
        user_id: &UserId,
        request_id: Option<String>,
    ) -> Result<(), HandleVoiceWebhookError> {
        let mut others: Vec<UserId> = self
            .call_repository
            .list_active(call_id)
            .await?
            .into_iter()
            .map(|p| p.user_id)
            .filter(|id| id != user_id)
            .collect();
        let mut index = 0;
        while index < others.len() {
            let other = others[index];
            let relation = self.friendship_repository.block_relation(user_id, &other).await?;
            if !relation.any() {
                index += 1;
                continue;
            }
            let (blocker, blocked) = if relation.blocking { (*user_id, other) } else { (other, *user_id) };
            let mut leaving = vec![blocker];
            if others.len() == 1 {
                leaving.push(blocked);
            }
            tracing::warn!(%call_id, blocker = %blocker.uuid(), blocked = %blocked.uuid(), "Blocked pair met in a call room; evicting");
            crate::usecases::call::eviction::evict(
                self.call_repository.as_ref(),
                self.voice_service.as_ref(),
                call_id,
                &leaving,
                request_id.clone(),
            )
            .await?;
            if leaving.contains(user_id) {
                return Ok(());
            }
            others.remove(index);
        }
        Ok(())
    }

    pub async fn execute(
        &self,
        body: &str,
        auth_token: &str,
        ctx: RequestContext,
    ) -> Result<(), HandleVoiceWebhookError> {
        let envelope = self.verifier.verify(body, auth_token)?;
        let source = envelope.source;
        let external_id = envelope.external_id;
        let occurred_at = envelope.occurred_at;

        let (channel_id, user_id, session_id, kind) = match envelope.event {
            VoiceWebhookEvent::ParticipantJoined {
                channel_id,
                user_id,
                session_id,
            } => {
                if !self.profiles.is_active(&UserId::new(user_id)).await? {
                    tracing::warn!(%channel_id, %user_id, "Inactive account joined a voice room; removing");
                    self.voice_service
                        .remove_participant(&ChannelId::new(channel_id), &UserId::new(user_id))
                        .await?;
                    return Ok(());
                }
                match self
                    .call_repository
                    .webhook_joined(
                        channel_id,
                        &UserId::new(user_id),
                        &session_id,
                        occurred_at,
                        ctx.request_id.clone(),
                    )
                    .await?
                {
                    WebhookJoinOutcome::Joined => {
                        self.evict_if_blocked(channel_id, &UserId::new(user_id), ctx.request_id.clone())
                            .await?;
                        return Ok(());
                    }
                    WebhookJoinOutcome::EvictionPending => {
                        tracing::warn!(%channel_id, %user_id, "Evicted participant reconnected to a call room; removing");
                        if let Err(e) = self
                            .voice_service
                            .remove_participant(&ChannelId::new(channel_id), &UserId::new(user_id))
                            .await
                        {
                            tracing::warn!(error = %e, %channel_id, %user_id, "Room removal failed; eviction stays queued");
                        }
                        return Ok(());
                    }
                    WebhookJoinOutcome::NotCallRoom => {}
                }
                (channel_id, user_id, session_id, WebhookKind::Joined)
            }
            VoiceWebhookEvent::ParticipantConnectionAborted {
                channel_id,
                user_id,
                session_id,
                reason,
            } => {
                if self
                    .call_repository
                    .webhook_left(
                        channel_id,
                        &UserId::new(user_id),
                        &session_id,
                        occurred_at,
                        ctx.request_id.clone(),
                    )
                    .await?
                {
                    return Ok(());
                }
                (channel_id, user_id, session_id, WebhookKind::Aborted(reason))
            }
            VoiceWebhookEvent::ParticipantLeft {
                channel_id,
                user_id,
                session_id,
                reason,
            } => {
                if self
                    .call_repository
                    .webhook_left(
                        channel_id,
                        &UserId::new(user_id),
                        &session_id,
                        occurred_at,
                        ctx.request_id.clone(),
                    )
                    .await?
                {
                    return Ok(());
                }
                (channel_id, user_id, session_id, WebhookKind::Left(reason))
            }
            VoiceWebhookEvent::TrackPublished {
                channel_id,
                user_id,
                session_id,
                source: media_source,
            } => {
                if self.call_repository.is_call_room(channel_id).await? {
                    return Ok(());
                }
                (
                    channel_id,
                    user_id,
                    session_id,
                    WebhookKind::Track {
                        source: media_source,
                        published: true,
                    },
                )
            }
            VoiceWebhookEvent::TrackUnpublished {
                channel_id,
                user_id,
                session_id,
                source: media_source,
            } => {
                if self.call_repository.is_call_room(channel_id).await? {
                    return Ok(());
                }
                (
                    channel_id,
                    user_id,
                    session_id,
                    WebhookKind::Track {
                        source: media_source,
                        published: false,
                    },
                )
            }
            VoiceWebhookEvent::Other => return Ok(()),
        };

        let event = WebhookEvent {
            source,
            external_id,
            channel_id,
            user_id,
            session_id,
            occurred_at,
            kind,
        };
        self.outbox.save(&event, ctx.request_id.as_deref()).await?;
        Ok(())
    }
}
