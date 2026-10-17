use contracts::events::webhook::{ConnectionAbortReasonV1, LeaveReasonV1, MediaSourceV1, WebhookEventV1};
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use uuid::Uuid;
use voice_domain::webhook::{ConnectionAbortReason, LeaveReason, MediaSource, WebhookEvent, WebhookKind};
pub(crate) use infrastructure_common::outbox::encoding::encode;

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


pub fn stream_limited_record(event: &voice_domain::stream::StreamLimited) -> ports::outbox::record::OutboxRecord {
    ports::outbox::record::OutboxRecord {
        id: uuid::Uuid::new_v4(),
        aggregate_id: event.user_id.uuid(),
        aggregate_type: ports::outbox::record::AggregateType::User,
        parent_id: None,
        parent_type: None,
        event_type: "stream_limited".to_string(),
        payload: serde_json::to_value(contracts::events::user::StreamLimitedV1 {
            user_id: event.user_id.uuid(),
            room_id: event.room.uuid(),
            track_sid: event.track_sid.clone(),
            width: event.ceiling.width,
            height: event.ceiling.height,
            frame_rate: event.ceiling.frame_rate,
            max_bitrate: event.ceiling.max_bitrate,
            occurred_at: event.occurred_at,
        })
        .unwrap_or_default(),
        occurred_at: event.occurred_at,
        request_id: None,
    }
}

#[cfg(test)]
mod stream_limited_tests {
    use domain::{channel::id::ChannelId, stream::StreamCeiling, user::id::UserId};

    use super::*;

    #[test]
    fn a_stream_limit_is_published_to_its_user_with_the_ceiling_and_track() {
        let event = voice_domain::stream::StreamLimited {
            user_id: UserId::new(uuid::Uuid::new_v4()),
            room: ChannelId::new(uuid::Uuid::new_v4()),
            track_sid: "TR_1".into(),
            ceiling: StreamCeiling::PRO,
            occurred_at: chrono::Utc::now(),
        };
        let record = stream_limited_record(&event);
        assert_eq!(record.event_type, "stream_limited");
        assert_eq!(record.aggregate_id, event.user_id.uuid());
        assert_eq!(record.payload["height"], 1440);
        assert_eq!(record.payload["track_sid"], "TR_1");
        assert_eq!(record.payload["room_id"], serde_json::json!(event.room.uuid()));
    }
}
