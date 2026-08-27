use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaveReason {
    ClientInitiated,
    Kicked,
    DuplicateIdentity,
    ServerShutdown,
    RoomClosed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaSource {
    Camera,
    Screen,
}

#[derive(Debug, Clone, Copy, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionAbortReason {
    SignalClose,
    MediaFailure,
    ConnectionTimeout,
    StateMismatch,
    JoinFailure,
    Unknown,
    Other,
}
