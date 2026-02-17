use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum TrackType {
    Audio,
    Video,
}
