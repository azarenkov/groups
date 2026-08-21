use platform::ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use contracts::events::{
    friendship::{
        FriendRequestAcceptedV1 as FriendAcceptedDto, FriendRequestSentV1 as FriendRequestSentDto,
        FriendshipRemovedV1 as FriendRemovedDto,
    },
    server::{ServerCreatedV1 as ServerCreatedDto, ServerDeletedV1 as ServerIdDto,
        ServerMemberV1 as ServerMemberDto},
    user::UserLifecycleV1 as UserNameDto,
};
use futures::StreamExt;
use tracing::Instrument;
use platform::messaging::nats::consumer::NatsEventConsumer;

use crate::recipients::RecipientResolver;

pub struct CatchupSpec {
    pub stream: String,
    pub consumer_name: String,
    pub filter_subject: String,
    pub dlq_subject: String,
}

pub async fn run_catchup(
    jetstream: async_nats::jetstream::Context,
    spec: CatchupSpec,
    resolver: RecipientResolver,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let consumer = NatsEventConsumer::new(jetstream, spec.stream.clone());
    loop {
        if *shutdown.borrow() {
            return;
        }
        let opts = SubscribeOptions {
            consumer_name: spec.consumer_name.clone(),
            subject: spec.filter_subject.clone(),
            start_position: StartPosition::All,
            max_deliver: Some(5),
            dlq_subject: Some(spec.dlq_subject.clone()),
        };
        match consumer.subscribe(opts).await {
            Ok(mut messages) => {
                tracing::info!(
                    stream = %spec.stream,
                    consumer = %spec.consumer_name,
                    "Projection catch-up consumer subscribed",
                );
                loop {
                    tokio::select! {
                        _ = shutdown.changed() => return,
                        msg = messages.next() => {
                            match msg {
                                Some(Ok(m)) => {
                                    match apply(&m.subject, &m.payload, &resolver)
                                        .instrument(tracing::info_span!(
                                            "event",
                                            request_id = ?m.request_id,
                                            subject = %m.subject,
                                        ))
                                        .await {
                                        Ok(()) => {
                                            if let Err(e) = m.ack().await {
                                                tracing::warn!(error = %e, "Projection catch-up ack failed");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(error = %e, "Projection catch-up failed; message will be redelivered");
                                        }
                                    }
                                }
                                Some(Err(e)) => {
                                    tracing::warn!(error = %e, "Projection catch-up stream error");
                                    break;
                                }
                                None => break,
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Projection catch-up subscribe failed; retrying");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

async fn apply(
    subject: &str,
    payload: &[u8],
    resolver: &RecipientResolver,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
