use contracts::events::webhook::{ConnectionAbortReasonV1, LeaveReasonV1, MediaSourceV1, WebhookEventV1};
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use uuid::Uuid;
use voice_domain::webhook::{ConnectionAbortReason, LeaveReason, MediaSource, WebhookEvent, WebhookKind};

pub fn to_record(event: &WebhookEvent, request_id: Option<&str>) -> Result<OutboxRecord, RepositoryError> {
    let (reason, media_source, published) = match event.kind {
        WebhookKind::Joined => (None, None, None),
        WebhookKind::Left(reason) => (Some(encode(&leave_reason(reason))?), None, None),
        WebhookKind::Aborted(reason) => (Some(encode(&abort_reason(reason))?), None, None),
        WebhookKind::Track { source, published } => (None, Some(media_source(source)), Some(published)),
    };
    let payload = WebhookEventV1 {
        channel_id: event.channel_id,
        user_id: event.user_id,
        session_id: event.session_id.clone(),
        occurred_at: event.occurred_at,
        inbox_source: event.source.as_str().to_string(),
        inbox_external_id: event.external_id.clone(),
        reason,
        media_source,
        published,
    };
    Ok(OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: event.channel_id,
        aggregate_type: AggregateType::Channel,
        parent_id: None,
        parent_type: None,
        event_type: event.event_type().to_string(),
        payload: encode(&payload)?,
        occurred_at: event.occurred_at,
        request_id: request_id.map(str::to_string),
    })
}

fn media_source(source: MediaSource) -> MediaSourceV1 {
    match source {
        MediaSource::Camera => MediaSourceV1::Camera,
        MediaSource::Screen => MediaSourceV1::Screen,
    }
}

fn leave_reason(reason: LeaveReason) -> LeaveReasonV1 {
    match reason {
        LeaveReason::ClientInitiated => LeaveReasonV1::ClientInitiated,
        LeaveReason::Kicked => LeaveReasonV1::Kicked,
        LeaveReason::DuplicateIdentity => LeaveReasonV1::DuplicateIdentity,
        LeaveReason::ServerShutdown => LeaveReasonV1::ServerShutdown,
        LeaveReason::RoomClosed => LeaveReasonV1::RoomClosed,
        LeaveReason::Unknown => LeaveReasonV1::Unknown,
    }
}

fn abort_reason(reason: ConnectionAbortReason) -> ConnectionAbortReasonV1 {
    match reason {
        ConnectionAbortReason::SignalClose => ConnectionAbortReasonV1::SignalClose,
        ConnectionAbortReason::MediaFailure => ConnectionAbortReasonV1::MediaFailure,
        ConnectionAbortReason::ConnectionTimeout => ConnectionAbortReasonV1::ConnectionTimeout,
        ConnectionAbortReason::StateMismatch => ConnectionAbortReasonV1::StateMismatch,
        ConnectionAbortReason::JoinFailure => ConnectionAbortReasonV1::JoinFailure,
        ConnectionAbortReason::Unknown => ConnectionAbortReasonV1::Unknown,
        ConnectionAbortReason::Other => ConnectionAbortReasonV1::Other,
    }
}

fn encode<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, RepositoryError> {
    serde_json::to_value(value).map_err(|e| RepositoryError::InvalidData(e.to_string()))
}
