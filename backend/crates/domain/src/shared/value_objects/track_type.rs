use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub enum TrackType {
    Audio,
    Video,
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid track type: expected 'audio' or 'video'")]
pub struct InvalidTrackType;

impl TrackType {
    pub fn from_string(s: &str) -> Result<Self, InvalidTrackType> {
        match s.to_lowercase().as_str() {
            "audio" => Ok(Self::Audio),
            "video" => Ok(Self::Video),
            _ => Err(InvalidTrackType),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }
}
