use chrono::{DateTime, Utc};
use domain::channel::webhook_source::WebhookSource;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaSource {
    Camera,
    Screen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaveReason {
    ClientInitiated,
    Kicked,
    DuplicateIdentity,
    ServerShutdown,
    RoomClosed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionAbortReason {
    SignalClose,
    MediaFailure,
    ConnectionTimeout,
    StateMismatch,
    JoinFailure,
    Unknown,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookKind {
    Joined,
    Left(LeaveReason),
    Aborted(ConnectionAbortReason),
    Track { source: MediaSource, published: bool },
}

#[derive(Debug, Clone)]
pub struct WebhookEvent {
    pub source: WebhookSource,
    pub external_id: String,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub session_id: String,
    pub occurred_at: DateTime<Utc>,
    pub kind: WebhookKind,
}

impl WebhookEvent {
    pub fn event_type(&self) -> &'static str {
        match self.kind {
            WebhookKind::Joined => "webhook_joined",
            WebhookKind::Left(_) => "webhook_left",
            WebhookKind::Aborted(_) => "webhook_aborted",
            WebhookKind::Track { .. } => "webhook_track",
        }
    }
}
